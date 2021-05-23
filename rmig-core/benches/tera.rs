use criterion::{black_box, criterion_group, criterion_main, Criterion};
use rmig_core::tera_manager::TeraManager;
use std::collections::HashMap;

fn simple_apply_context_and_resolve_vars(
    pattern: &str,
    env: HashMap<String, String>,
) -> anyhow::Result<()> {
    let result = TeraManager::new(env).apply("any", pattern)?;
    assert_eq!("SELECT WORLD FROM DUAL;", result.as_str());
    Ok(())
}

fn tera_apply_context_and_resolve(c: &mut Criterion) {
    c.bench_function("Testing tera resolving.", |b| {
        b.iter(|| {
            let mut env = HashMap::<String, String>::new();
            env.insert(String::from("name"), String::from("WORLD"));
            env.insert(String::from("table"), String::from("DUAL"));
            simple_apply_context_and_resolve_vars("SELECT {{ name }} FROM {{ table }};", env)
                .unwrap();
        })
    });
}

criterion_group!(benches, tera_apply_context_and_resolve);
criterion_main!(benches);
