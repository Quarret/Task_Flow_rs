use std::{fmt, sync::{Arc, Mutex}, thread, time::Duration};

const COLOR_REST: &str = "\x1b[0m";
const COLOR_RED: &str = "\x1b[31m";
const COLOR_GREEN: &str = "\x1b[32m";
const COLOR_YELLOW: &str = "\x1b[33m";

// 自定义错误类型
// dervie 自动实现 trait ("接口")
#[derive(Debug)]
enum TaskError {
    ExecutionError(String), // 运行错误
    TimeOut,
    NotFound,
}

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

// 自动实现 Priority 的 格式打印 {:?}, .clone(), 逻辑判断 == / !=
#[derive(Debug, Clone, PartialEq)]
enum Priority {
    High,
    Medium,
    Low,
}

// 特性: 接口
// send: 所有权可以转移 sync: 可以被多线程共享
trait Executable: Send + Sync {
    fn execute(&self) -> Result<(), TaskError>;
    fn get_name(&self) -> String;
}

struct SimpleTask {
    name: String,
    duration_secs: u64,
}

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

// Arc: 可以多线程共享  Box: 一个指向堆分配内存的指针
// Mutex: 互斥锁 Vec: 要求每个元素大小固定
struct Scheduler {
    tasks: Arc<Mutex<Vec<(Priority, Box<dyn Executable>)>>>,
}

impl Scheduler {
    // 创建
    fn new() -> Self {
        Scheduler { 
            tasks: Arc::new(Mutex::new(Vec::new())), 
        }
    }

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
}

// 随机生成任务 
fn random_task(scheduler: &Scheduler) {
    let task_name = vec![
        "系统扫描", "数据同步", "邮件发送", "缓存清理", 
        "安全审计", "日志压缩", "前端构建", "AI 模型推理",
    ];

    // 按照系统时间生成随机种子
    use std::time::SystemTime;
    let mut seed = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let m = 10;
    println!("--- 开始随机生成 {} 个任务", m);

    for i in 0..m {
        seed = (seed * 1103515245 + 12345) & 0x7fffffff; // 简单的线性同余伪随机

        // 随机选择
        let name_idx = (seed as usize) % task_name.len();
        let name = format!("第 {} 个任务 - {}", i, task_name[name_idx]);

        // 随机优先级
        let priority = match seed % 3 {
            0 => Priority::High,
            1 => Priority::Medium,
            _ => Priority::Low
        };

        // 随机持续时间
        let limit = 10;
        let duration = (seed % limit) + 1;

        let task = Box::new(SimpleTask {
            name,
            duration_secs: duration
        });

        println!("{}已添加任务: {} {} | 优先级: {:?} | 预估时间: {}s", COLOR_YELLOW, COLOR_REST, task.get_name(),  priority, duration);
        scheduler.add_task(priority, task);
    }

    println!();
}

fn main() {
    println!("--- TaskFlow 开始 ---");
    println!();

    let scheduler = Scheduler::new();

    random_task(&scheduler);

    scheduler.run_all();
}
