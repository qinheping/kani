- **Feature Name:** Loop-contract synthesis
- **Feature Request Issue:** <https://github.com/model-checking/kani/issues/2214>
- **RFC PR:** <https://github.com/model-checking/kani/pull/2215>
- **Status:** Under Review
- **Version:** 0
- **Proof-of-concept:** <https://github.com/qinheping/kani/tree/kani-synthesizer>

## Summary

A new option that allows users to verify programs without unwinding loops by synthesizing loop contracts for those loops.


## User Impact
Currently Kani does not support proving correctness (i.e. assertions never fail) on programs with unbounded control flow (e.g. loops with dynamic bounds). Kani unrolls all unbounded loops until a global threshold (unwinding number) specified by the user and then verifies this unrolled program, which limits the set of programs it can verify.

A new Kani flag `--synthesize-loop-contracts` will be created that can be used to enable the goto-level loop-contract synthesizer `goto-synthesizer`.
The idea of [loop contracts](https://arxiv.org/pdf/2010.05812.pdf) is, instead of unwinding loops, we abstracts those loops as non-loop structures that can cover arbitrary iterations of the loops.
The loop contract synthesizer, when enabled, will attempt to synthesize loop contracts for all loops.
CBMC can then apply the synthesized loop contracts and verify the program without unwinding any loop.
So, the synthesizer will help to verify the programs that require Kani to unwind loops for a very large number of times to cover all iterations.


For example, the number of executed iterations of the loop in the following harness is dynamically bounded by an unbounded variable `y`.
Only the unwinding number `i32::MAX` can guarantee to cover all iteration of the loop, and hence satisfies the unwinding assertions.
Unwinding the loop `i32::MAX` number of time will result in a too large goto program to be verified by CBMC.  
```rust
#[kani::proof]
fn main() {
    let mut y: i32 = kani::any_where(|i| *i > 0);

    while y > 0 {
        y = y - 1;
    }
    assert!(y == 0);
}
```
With the loop-contract synthesizer, Kani can synthesize the loop invariant `y >= 0`, with which it can prove the post-condition `y == 0` without unwinding the loop.


Also, loop contracts could also improve Kani’s verification time since all loops will be abstracted to a single iteration, as opposed to being unwound a large number of iterations.
For example, we can easily find out that the following loop is bounded by the unwinding number `5000`.
Kani can verify the program in a few minutes by unwinding the loop 5000 times.
With loop contracts, we only need to verify the single abstract iteration of the loop, which leads to a smaller query.
As a result, Kani with the synthesizer can verify the program in a few seconds.
```rust
#[kani::proof]
#[kani::unwind(5000)]
fn main() {
    let mut y: i32 = 5000;

    while y > 0 {
        y = y - 1;
    }
    assert!(y == 0);
}
```

The `goto-synthesizer` is an [enumeration-based synthesizer](https://www.cis.upenn.edu/~alur/SyGuS13.pdf).
It enumerates candidate invariants from a pre-designed search space described by a given regular tree grammar and verifies if the candidate is an inductive invariant.
Therefore it has the following limitations:
1. the search space is not complete, so it may fail to find a working candidate. The current search space consists of only conjunction of linear inequalities built from the variables in the loop, which is not expressive enough to capture all loop invariants.
For example, the loop invariant `a[i] == 0` contains array access and cannot be captured by the current search space.
However, we can easily extend the search space to include more complex expressions with the cost of an exponential increase of the running time of the synthesizer.
2. the synthesizer suffers from the same limitation as the loop contract verification in CBMC. For example, it dose not support unbounded quantifiers, or dynamic allocations in the loop body. 

## User Experience

Once this RFC has been stabilized, users will be able to synthesize loop contracts for all loops and verify programs with the synthesized loop contracts with a new command line flag `--synthesize-loop-contracts`.


#### Limit Resource Used by Synthesizer for Termination
Without a resource limit, an enumerative synthesizer may run forever to exhaust a search space consisting of an infinite number of candidates, especially when there is no solution in the search space.
So, for the guarantee of termination, we provide users options: `--limit-synthesis-time T` to limit the running time of the synthesizer.


#### Output of Kani when the Synthesizer is Enabled
When the flag `--synthesize-loop-contracts` is provided, Kani will report different result for different cases
1. When there exists some loop invariant in the candidate space with which all assertions can be proved, Kani will synthesize the loop contracts, verify the program with the synthesized loop contracts, and report verification SUCCESS;
2. When no working candidate has been found in the search space within the specified limits, Kani will report the verification result with the best-effort-synthesized loop contracts.
Note that as loop contracts are over-approximations of the loop, the violated assertions in this case may be spurious.
So we will report the violated assertions as `UNDETERMIN` instead of `FAILED`.

A question about how do we print the synthesized loop contracts when users request is discussed in **Open question**.

## Detailed Design
The synthesizer ```goto-synthesizer``` is implemented in the repository of `CBMC`, takes as input a goto binary, and outputs a new goto binary with the synthesized loop contracts applied.
Currently, Kani invokes `goto-instrument` to instrument the goto binary `main.goto` into a new goto binary `main_instrumented.goto`, and then invokes ```cbmc``` on `main_instrumented.goto` to get the verification result.
The synthesis will happen between calling `goto-instrument` and calling ```cbmc```.
That is, we invoke ```goto-synthesizer``` on ```main_instrumented.goto``` to produce a new goto binary ```main_synthesized.goto```, and then call ```cbmc``` on `main_synthesized.goto` instead. 

When invoking ```goto-synthesizer```, we pass the following parameters to it with the flags built in `goto-synthesizer`:
- the resource limit of the synthesis;
- the solver options to specify what SAT solver we use to verify invariant candidates.

The enumerator used in the synthesizer enumerates candidates from the language of the following grammar template
```
NT_Bool -> NT_Bool && NT_Bool | NT_int == NT_int 
            | NT_int <= NT_int | NT_int < NT_int 
            | SAME_OBJECT(terminals_ptr, terminals_ptr)
            
NT_int  -> NT_int + NT_int | terminals_int | LOOP_ENTRY(terminals_int)
            | POINTER_OFFSET(terminals_ptr) | OBJECT_SIZE(terminals_ptr)
            | POINTER_OFFSET(LOOP_ENTRY(terminals_ptr)) | 1
```
where `terminals_ptr` are all pointer variables in the scope, and `terminal_int` are all integers variables in the scope.

We use the following example to illustrate how the synthesizer works.
```rust
#[kani::proof]
fn main() {
    let mut y: i32 = kani::any_where(|i| *i > 0);

    while y > 0 {
        y = y - 1;
    }
    assert!(y == 0);
}
```
As there is only one variable `y` in the scope, the grammar template above will be instantiated to the following grammar
```
NT_Bool -> NT_Bool && NT_Bool | NT_int == NT_int 
            | NT_int <= NT_int | NT_int < NT_int 
NT_int  -> NT_int + NT_int | y | LOOP_ENTRY(y) | 1
```
The synthesizer will enumerate candidates derived from `NT_Bool` in the following order.
```
y == y
y == LOOP_ENTRY(y)
y == 1
...
1 <= y + 1
...
```
The synthesizer then verifies with CBMC if the candidate is an inductive invariant that can be used to prove the post-condition `y == 0`.
For example, the candidate `y == y` is verified to be an inductive invariant, but cannot be used to prove the post-condition `y == 0`.
The candidate `y == 1` is not inductive.
The synthesizer rejects all candidates until it finds the candidate `1 <= y + 1`, which can be simplified to `y >= 0`.
`y >= 0` is an inductive invariant that can be used to prove the post-condition.
So the synthesizer will return `y >= 0` and apply it in the goto model to get `main_synthesized.goto`.
Then Kani will call `cbmc` on `main_synthesized.goto` to verify the program with the synthesized loop contracts. 

## Rationale and alternatives

- Different candidate space.
The candidate grammar introduced above now only contains a restricted set of operators, which works well for array-manipulating programs with only pointer-checks instrumented by `goto-instrument`, but probably not enough for other user-written checks.
We may want to include array-indexing, pointer-dereference, or other arithmetic operators in the candidate grammar for synthesizing a larger set of loop invariants.
However, there is a trade-off between the size of candidate we enumerate and the running time of the enumeration.
We will collect more data to decide what operators we should include in the candidate grammar.

## Open questions

**How does the synthesizer work with unwinding numbers?**
There may exist some loops for which the synthesizer cannot find loop contracts, but some small unwinding numbers are enough to cover all executions of the loops.
In this case, we may want to unwind some loops in the program while synthesizing loop contracts for other loops.
It requires us to have a way to identify and specify which loops we want to unwind. 

In C programs, we identify loops by **loop id**, which is a pair (function name, loop number).
However, it Rust programs, loops are usually in library functions such as `Iterator::for_each`.
And a library function may be called from different places in the program.
We may want to unwind the loop in some calls but not in other calls.

**How do we output the synthesized loop contracts?**
To better earn users' trust, we want to be able to report what loop contracts we synthesized and used to verify the given programs.
Now `goto-synthesizer` can dump the synthesized loop contracts into a json file.
Here is an example of the dumped loop contracts.
It contains the location of source files of the loops, the synthesized loop invariants and loop assigns for loops identified by loop numbers.
```json
{
    "sources": [ "/Users/qinhh/Repos/playground/kani/synthesis/base_2/test.rs" ],
    "functions": [
      {
        "main": [ "loop 1 invariant y >= 0", 
                  "loop 1 assigns var_9,var_10,var_11,x,y,var_12" ]
      }
    ],
    "output": "stdout"
}
```

There are two challenges here if we want to also dump synthesized loop contracts in Kani.
1. We need to have a consistent way to identify loops.
2. We need to dump loop invariants in `rust` instead of `c`.
3. There are many auxiliary variables we added in Kani-compiled GOTO, such as `var_9`, `var_10`, `var_11`, and `var_12` in the above json file.
We need to translate them back to the original variables they represent.


## Future possibilities
If we have a good answer for how to identify loops and dump synthesized loop contracts, we could probably also allow users to provide the loop contracts they wrote to Kani, and verify programs with user-provided loop contracts.