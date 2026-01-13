use criterion::BatchSize;
use criterion::BenchmarkId;
use criterion::Throughput;
use criterion::{criterion_group, criterion_main, Criterion};
use lambo::ast::AST;

fn benchmark_ast(benchmark_name: &str, input: usize) -> AST {
    let lib = include_str!("./benchmarks.lambo");
    let source = format!("{lib} {benchmark_name} {input}");
    let mut ast = AST::from_str(&source);
    ast.garbage_collect();
    ast
}

fn numbers_stream(c: &mut Criterion) {
    let mut group = c.benchmark_group("numbers_from");
    for size in (8..12).map(|exp| (2 as usize).pow(exp)) {
        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, _| {
            let ast = benchmark_ast("bench_numbers", size);
            b.iter_batched(
                || ast.clone(),
                |mut ast| {
                    ast.evaluate(ast.root).unwrap();
                },
                BatchSize::SmallInput,
            );
        });
        group.bench_with_input(BenchmarkId::new("Native", size), &size, |b, &size| {
            b.iter(|| native::bench_numbers(size, 0));
        });
    }
    group.finish();
}

fn primes_stream(c: &mut Criterion) {
    let mut group = c.benchmark_group("primes");
    for size in (5..8).map(|exp| (2 as usize).pow(exp)) {
        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, _| {
            let ast = benchmark_ast("bench_primes", size);
            b.iter_batched(
                || ast.clone(),
                |mut ast| {
                    ast.evaluate(ast.root).unwrap();
                },
                BatchSize::SmallInput,
            );
        });
    }
    group.finish();
}

criterion_group!(benches, numbers_stream, primes_stream);
criterion_main!(benches);

/// Mirrors native implementations from benchmarks.lambo
/// The goal is to answer the question:
/// "If we compiled lambo to Rust (or C), how much faster can we get?"
mod native {
    type Thunk<T> = Box<dyn Fn() -> T>;

    #[allow(dead_code)]
    enum List {
        Cons(usize, Thunk<List>),
        Nil,
    }

    fn numbers_from(start: usize) -> List {
        List::Cons(start, Box::new(move || numbers_from(start + 1)))
    }
    fn nth(list: List, index: usize) -> usize {
        match list {
            List::Nil => panic!("OUT OF BOUNDS"),
            List::Cons(head, tail) => {
                if index == 0 {
                    return head;
                }
                return nth(tail(), index - 1);
            }
        }
    }

    pub fn bench_numbers(n: usize, start: usize) -> usize {
        return nth(numbers_from(start), n);
    }
}
