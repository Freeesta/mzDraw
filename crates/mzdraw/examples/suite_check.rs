//! The check of one task of the agent task suite: `suite_check TASK.json REQUEST.json [TASK_ID]` reads a task (a task object, or the
//! whole `agent-suite-tasks/1` file with the id of the task as third argument) and a replay request, runs the request and the check
//! of the task and prints `{"task","pass","reason"}` on one line. Exit 0 pass, 1 fail, 2 a file or an argument that cannot be read.

#[path = "../tests/suite/check.rs"]
mod check;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let read = |p: &String| std::fs::read_to_string(p);
    let (task, request) = match (args.first().map(read), args.get(1).map(read)) {
        (Some(Ok(t)), Some(Ok(r))) if args.len() <= 3 => (t, r),
        _ => {
            eprintln!("usage: suite_check TASK.json REQUEST.json [TASK_ID]");
            std::process::exit(2);
        }
    };
    let Ok(mut task) = serde_json::from_str::<serde_json::Value>(&task) else {
        eprintln!("suite_check: the task file is not JSON");
        std::process::exit(2);
    };
    if let Some(id) = args.get(2) {
        let found = task["tasks"]
            .as_array()
            .and_then(|l| l.iter().find(|t| t["id"] == id.as_str()))
            .cloned();
        match found {
            Some(t) => task = t,
            None => {
                eprintln!("suite_check: no task {id}");
                std::process::exit(2);
            }
        }
    }
    let Some(id) = task["id"].as_str().map(str::to_string) else {
        eprintln!("suite_check: the task has no id");
        std::process::exit(2);
    };
    let result = check::check(&task, &request);
    println!("{}", check::report(&id, &result));
    std::process::exit(i32::from(result.is_err()));
}
