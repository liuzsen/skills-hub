//! Inject / InjectSingleton / inject / inject_singleton 用法示例。
//! 运行：cargo run --example inject --features inject
use lolibaso::inject::{Inject, InjectSingleton, TypeBuilder, inject, inject_singleton};
use std::{
    future::Future,
    sync::Arc,
    task::{Context, Poll, Waker},
};

struct Repository(Arc<String>);

// 每个 impl 恰好一个无 self 的构造函数；方法名不限于 new。
#[inject]
impl Repository {
    fn open(#[instance_clone] name: Arc<String>) -> Self {
        Self(name)
    }
}

// 泛型依赖递归构造；具体类型依赖默认从 builder 移出。
// derive 也支持元组 / unit struct，不支持 enum / union。
#[derive(Inject)]
struct Service<R> {
    repository: R,
    port: u16,
    #[inject(instance_clone)]
    name: Arc<String>,
    #[inject(default)]
    label: String,
}

struct App<R> {
    repository: R,
    name: String,
}

#[inject]
impl<R> App<R> {
    // impl 泛型依赖递归构造；函数泛型参数从 builder 取实例。
    fn new<N>(repository: R, name: N) -> Self
    where
        N: Into<String>,
    {
        Self {
            repository,
            name: name.into(),
        }
    }
}

// 单例要求 Clone + 'static；同一 builder 内缓存，具体依赖从 builder 克隆。
#[derive(InjectSingleton, Clone)]
struct Cache {
    #[inject(default)]
    identity: Arc<()>,
}

#[derive(Clone)]
struct Config(u16);

#[inject_singleton]
impl Config {
    // 同步构造器生成同步和异步注入实现；async 构造器只生成异步实现。
    // 可返回 Self 或 Result<Self, E>；错误传播给 build / build_async。
    async fn load(port: u16) -> std::io::Result<Self> {
        if port == 0 {
            return Err(std::io::Error::other("port must not be zero"));
        }
        Ok(Self(port))
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let name = Arc::new("users".to_owned());
    let builder = TypeBuilder::new()
        .add_instance(8080_u16)
        .add_instance(name.clone());
    let (service, first, second) = builder.build::<(Service<Repository>, Cache, Cache), _>()?;
    let Service {
        repository,
        port,
        name: retained,
        label,
    } = service;
    assert!(Arc::ptr_eq(&repository.0, &name));
    assert!(Arc::ptr_eq(&retained, &name));
    assert_eq!(port, 8080);
    assert!(label.is_empty());
    assert!(Arc::ptr_eq(&first.identity, &second.identity));
    let independent = TypeBuilder::new().build::<Cache, _>()?;
    assert!(!Arc::ptr_eq(&first.identity, &independent.identity));

    // 函数泛型位于注入索引元组前部，其余依赖索引交给推断。
    let app = TypeBuilder::new()
        .add_instance(name.clone())
        .add_instance("admin")
        .build::<App<Repository>, (&str, _, _)>()?;
    assert!(Arc::ptr_eq(&app.repository.0, &name));
    assert_eq!(app.name, "admin");

    let (first, second) = run_ready(
        TypeBuilder::new()
            .add_instance(8080_u16)
            .build_async::<(Config, Config), _>(),
    )?;
    assert_eq!((first.0, second.0), (8080, 8080));
    assert!(
        run_ready(
            TypeBuilder::new()
                .add_instance(0_u16)
                .build_async::<Config, _>()
        )
        .is_err()
    );
    Ok(())
}

fn run_ready<F: Future>(future: F) -> F::Output {
    match std::pin::pin!(future).poll(&mut Context::from_waker(Waker::noop())) {
        Poll::Ready(output) => output,
        Poll::Pending => panic!("example unexpectedly suspended"),
    }
}

// derive 字段支持属性：default, instance_clone；
// 构造参数只支持裸 #[instance_clone]，泛型依赖不支持 instance_clone。
