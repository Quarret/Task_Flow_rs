# Rust 版任务调度器
这个项目的主要目的是来练习 `rust` 基本语法与特性的, 其中包含 `特征` `枚举`, `闭包`, `并发`等特性. 

下面从 `自定义任务错误`, `任务`, `任务调度` 三个角度解读代码

## 自定义任务错误
通过 `枚举` 实现多个 `任务错误`, 上方的 `derive` 是自动实现宏. 
例如 `Debug` 实现直接 `{:?}` 输出, `Clone` 实现 `.clone()` 函数, `PartialEq` 实现 `==` 和 `!=` 逻辑判断

```rust
// 自定义错误类型
// dervie 自动实现 trait ("接口")
#[derive(Debug)]
enum TaskError {
    ExecutionError(String), // 运行错误
    TimeOut,
    NotFound,
}
```

单个 `Debug` 实现的错误信息报错过于简陋, 通过重写 `fmt::Display` 实现更精细的问题输出.
```rust
// 为枚举类 TaskError 实现 fmt::Display
impl fmt::Display for TaskError {
    // &self: taskerror 不可变引用  &mut: 可变引用 <'_>: 生命周期为这个函数 f = formatter: 格式化工具
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TaskError::ExecutionError(msg) => write!(f, "执行任务失败: {}", msg),
            TaskError::TimeOut => write!(f, "任务超时"),
            TaskError::NotFound => write!(f, "找不到任务")
        }
    }
}
```

这里来说明一下 `rust` 的一些语法
1. `fn fmt(...) -> ...`, 这是函数写法, 括号中为参数 (`参数名: 参数类型`), `->` 为返回值类型
2. `&` 与 `&mut`, 不可变借用和可变借用. 由于 `rust` 采用的是 `借用模型`, 要注意是否可变
3. `match 参数名 {参数值 => ...}`, `match` 可以与之前的 `枚举` 一起使用, 相当于 `switch`
4. `<'_>`, 限制修饰参数的生命周期时限定生命周期为 `当前函数`

## 任务
特征 `trait`, 相当于 `接口`, 可以为类来实现 `trait`
```rust
// 特性: 接口
// send: 所有权可以转移 sync: 可以被多线程共享
trait Executable: Send + Sync {
    fn execute(&self) -> Result<(), TaskError>;
    fn get_name(&self) -> String;
}
```

其中的 `send` 和 `sync` 修饰词, 代表该特征 `所有权可以转移` 和 `可以被多线程共享`

实现 `trait` 的语法:
```rust
// 为 simpletask 实现 executable 特性, result 是枚举
impl Executable for SimpleTask {
    fn execute(&self) -> Result<(), TaskError> {
        println!("正在运行任务: {}", self.name);
        if self.duration_secs > 5 {
            return Err(TaskError::ExecutionError("任务需要运行时间过长, 系统拒绝".to_string()));
        }

        thread::sleep(Duration::from_secs(self.duration_secs));
        Ok(())
    }

    fn get_name(&self) -> String {
        self.name.clone()
    }
}
```

里面的 `Result<T, E>` 为枚举, 传递进来的分别是 `OK()` 和 `Err` 的参数. 
例如 `Result<(), TaskError>` 返回正常运行为 `OK(())`, 返回错误为 `Err(TaskError::)`

## 调度器
调度器可能被 `多个线程` 持有, 所以需要它满足原子性且不会出现并发错误
```rust
// Arc: 可以多线程共享  Box: 一个指向堆分配内存的指针
// Mutex: 互斥锁 Vec: 要求每个元素大小固定
struct Scheduler {
    tasks: Arc<Mutex<Vec<(Priority, Box<dyn Executable>)>>>,
}
```

1. `Arc`: Atomic Reference counter. 原子引用计数, 只要还有一个线程持有, 数据就不会被销毁
2. `Mutex`: 互斥锁, 保证并发性
3. `Box`: 一个指向堆内存的指针, 由于 `Vec` 需要元素大小固定, 所以将 `task` 通过 `Box` "装箱"

### 添加任务
```rust
// 添加任务
fn add_task(&self, priority: Priority, task: Box<dyn Executable>) {
    let mut tasks = self.tasks.lock().unwrap();
    tasks.push((priority, task));
    tasks.sort_by(|a, b| {
        let priority_val = |p: &Priority| match p {
            Priority::High => 0,
            Priority::Medium => 1,
            Priority::Low => 2,
        };
        // a.0 = a.Priority
        priority_val(&b.0).cmp(&priority_val(&a.0))
    });
} 
```

1. `添加任务` 需要改变调度器中的 `Vec`, 需要其可变, 用 `mut` 修饰实现. 
2. `lock()`, 利用 `mutex` 互斥锁保证不会出现并发问题
3. `unwrap()`, 当 `程序` 运行中出现错误终止程序并报错 `Err()`
4. `闭包比较函数`, `sort_by(|a, b| {...})`, 相当于 `sort(..., [](auto &x, auto& y) {return ...})`

### 并发处理任务
```rust
// 并发处理任务, 主线程(调度器) 其他线程(处理任务)
fn run_all(self) {
    // 调度器中需要处理的任务
    let task_arc = Arc::clone(&self.tasks);

    // 开启新线程来处理
    let handle = thread::spawn(move || {
        // 可变 tasks
        let mut tasks = task_arc.lock().unwrap();
        println!("--- 调度器开始工作, 待处理任务总数: {}", tasks.len());
        println!();

        // 使用迭代器处理
        while let Some((priority, task)) = tasks.pop() {
            println!("{}[{:?}]{} 准备运行: {}",  match priority {
                Priority::High => COLOR_RED, 
                Priority::Medium => COLOR_YELLOW, 
                Priority::Low => COLOR_GREEN, 
            },  priority, COLOR_REST, task.get_name());

            match task.execute() {
                Ok(_) => println!("{}Successfully Finished: {}{}", COLOR_GREEN,  COLOR_REST, task.get_name()),
                Err(e) => eprintln!("{}Error running :{} {} {}", COLOR_RED, COLOR_REST, task.get_name(), e),
            }
            println!();
        }

        println!("--- 所有任务执行完毕 ---");
    });

    handle.join().unwrap();
}
```

1. `Arc::clone` 的原因是主线程可能比开启的新线程存活时间更短, 如果调度器被销毁就会发生错误, 利用 `Arc` 复制一个指向调度器内存的指针, 引用数 ＋1, 保证调度器在运行时不被销毁
2. `Some(...)` 是一种模式匹配, 当匹配不成功时, 结束循环; 成功时, 将数据包分配给两个参数, 相当于 `auto [..., ...]`

## 任务初始化
通过上述学习就可以看懂 `任务初始化` 代码, 随机化种子那不是重点, 也可以用其他随机方法实现 

## 项目架构流程图
```Mermaid
graph TD
    A([程序启动 main]) --> B[初始化调度器 Scheduler::new]
    B --> C[调用 random_task 生成任务]
    
    subgraph 任务生成阶段
    C --> D{循环生成 10 个任务}
    D -->|生成随机参数| E[创建 SimpleTask]
    E -->|确定优先级| F[调用 scheduler.add_task]
    F --> G[获取互斥锁 Mutex]
    G --> H[将任务推入 Vec]
    H --> I[根据优先级排序 sort_by]
    I -->|释放锁| D
    end

    D -->|循环结束| J[调用 scheduler.run_all]
    
    subgraph 任务执行阶段
    J --> K[克隆 Arc 指针]
    K --> L[开启新线程 thread::spawn]
    L --> M[获取互斥锁 Mutex]
    M --> N{Vec 中还有任务吗? while let Some}
    
    N -->|有任务| O[弹出任务 pop]
    O --> P[打印准备运行信息]
    P --> Q[执行任务 task.execute]
    Q --> R{执行结果 Result}
    R -->|Ok| S[打印成功信息]
    R -->|Err| T[打印错误信息]
    S --> N
    T --> N
    
    N -->|无任务| U[打印所有任务执行完毕]
    end

    U --> V[主线程等待子线程结束 join]
    V --> W([程序结束])
```

## 所有权 Arc 和 互斥锁 Mutex 的时序图
``` Mermaid
sequenceDiagram
    participant Main as 主线程 (main)
    participant Scheduler as 调度器 (Scheduler)
    participant Mutex as 互斥锁 (Mutex<Vec>)
    participant Worker as 工作线程 (Thread)
    participant Task as 任务 (SimpleTask)

    Note over Main: 1. 初始化
    Main->>Scheduler: new()
    Scheduler-->>Main: 返回实例

    Note over Main: 2. 生成任务
    loop 10次
        Main->>Main: 生成随机数据
        Main->>Scheduler: add_task(priority, task)
        Scheduler->>Mutex: lock()
        Mutex-->>Scheduler: 获取数据访问权
        Scheduler->>Scheduler: push() & sort()
        Scheduler->>Mutex: unlock() (自动释放)
    end

    Note over Main: 3. 执行任务
    Main->>Scheduler: run_all()
    Scheduler->>Worker: thread::spawn()
    
    activate Worker
    Worker->>Mutex: lock()
    Mutex-->>Worker: 获取数据访问权
    
    loop 直到 Vec 为空
        Worker->>Worker: tasks.pop()
        alt 有任务
            Worker->>Task: execute()
            alt duration > 5s
                Task-->>Worker: Err(ExecutionError)
            else duration <= 5s
                Task-->>Worker: Ok(())
            end
            Worker->>Worker: 打印结果
        else 无任务
            Worker->>Worker: 退出循环
        end
    end
    
    Worker->>Mutex: unlock() (自动释放)
    deactivate Worker
    
    Worker-->>Main: join() 返回
    Note over Main: 程序结束
```

## 问题 & 未来展望
未测试本代码在更多 `线程` 下的并发安全性, 未实现 `任务错误` 中的 `Timeout` 和 `NotFound`

TODO
- [ ] 利用更多 `线程` 来实现任务
- [ ] 对于超时任务, 利用 `RR` 策略完成
- [ ] 实现 `TimeOut` 和 `NotFound` 任务错误


