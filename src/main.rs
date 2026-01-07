use termion::{
    raw::IntoRawMode,
    cursor,
    clear,
    terminal_size,
    input::TermRead,
    event::Key,
};
use std::{
    process,
    fs,
    io::{stdout, Write, Stdout, self},
};
use colored::Colorize;
use serde::{Serialize, Deserialize};
use chrono::Local;

const BORDER: u16 = 1;
const HEADING: u16 = 3;
const CMDLINE: u16 = 1;
const SCROLL_PADDING: u16 = 5;
const DATA_FILE: &str = "data.json";

enum Command {
    Quit,
    Add,
    DeleteTask,
    SwitchState,
    MoveDown,
    MoveUp,
    Invalid,
    InsertChar(char),
    DeleteChar,
    EnterNormalMode,
    EnterCommand,
}

#[derive(Debug, Serialize, Deserialize)]
enum State {
    Todo,
    Doing,
    Done,
}
impl State {
    fn colour(&self) -> colored::ColoredString {
        match self {
            State::Todo => "Todo".white(),
            State::Doing => "Doing".yellow(),
            State::Done => "Done".green(),
        }
    }
}

#[derive(Debug)]
enum Mode {
    Normal,
    Adding,
}

#[derive(Debug, Serialize, Deserialize)]
struct Task {
    name: String,
    created_at: String,
    completed_at: String,
    state: State,
}

struct Global {
    terminal_w: u16,
    terminal_h: u16,
    command_line: String,
    cur_row: u16,
    start: u16,
    end: u16,
    mode: Mode,
}
impl Global {
    fn update_terminal_size(&mut self) {
        let (w, h) = terminal_size().unwrap_or((20, 20));
        self.terminal_w = w;
        self.terminal_h = h;
    }
}

fn draw_grid(global: &Global, stdout: &mut Stdout) -> Result<(), io::Error>{
    //top row
    for i in 0..global.terminal_w {
        write!(stdout, "{}", cursor::Goto(i + 1, 1))?;
        if i == 0 {
            write!(stdout, "╔")?;
        }
        else if i >= global.terminal_w - 1 {
            write!(stdout, "╗")?;
        }
        else {
            write!(stdout, "═")?;
        }
    }
    //bottom row
    for i in 0..global.terminal_w {
        write!(stdout, "{}", cursor::Goto(i + 1, global.terminal_h - CMDLINE))?;
        if i == 0 {
            write!(stdout, "╚")?;
        }
        else if i >= global.terminal_w - 1 {
            write!(stdout, "╝")?;
        }
        else {
            write!(stdout, "═")?;
        }
    }
    //left side
    for i in 1..global.terminal_h - 1 - CMDLINE {
        write!(stdout, "{}", cursor::Goto(1, i + 1))?;
        write!(stdout, "║")?;
    }
    //rigth side
    for i in 1..global.terminal_h - 1 - CMDLINE {
        write!(stdout, "{}", cursor::Goto(global.terminal_w, i + 1))?;
        write!(stdout, "║")?;
    }

    Ok(())
}

fn print_tasks(tasks: &Vec<Task>, stdout: &mut Stdout, global: &Global) -> Result<(), io::Error> {
    let step = (global.terminal_w - BORDER) / 4;
    let mut y = HEADING;
    for i in global.start..global.end{
        let task = match tasks.get(i as usize){
            Some(t) => t,
            None => break,
        };

        if i as u16 == global.cur_row {
            write!(stdout, "{}{}", cursor::Goto(step, y as u16 + 2), task.created_at.as_str().blue())?;

            write!(stdout, "{}{}", cursor::Goto(step * 2, y as u16 + 2), task.completed_at.as_str().blue())?;
        }
        else {
            write!(stdout, "{}{}", cursor::Goto(step, y as u16 + 2), task.created_at)?;

            write!(stdout, "{}{}", cursor::Goto(step * 2, y as u16 + 2), task.completed_at)?;
        }

        write!(stdout, "{}{}", cursor::Goto(step * 3, y as u16 + 2), task.state.colour())?;
        
        //make it so if the name is too long we print it in parts underneath each other
        if task.name.len() as u16 > step - 1 {
          
           //split the string into multiple sub-strings
           let mut amount = 0;
           let mut subs: Vec<String> = Vec::new();
           let mut new_sub = String::new();
           let mut index = 0;

           for c in task.name.chars() {
                if index >= step as usize - 3 {
                    subs.push(new_sub.clone());
                    new_sub.clear();
                    amount += 1;
                    index = 0;
                    new_sub.push(c);
            
                }
                else {
                    index += 1;
                    new_sub.push(c);
                }
           }
           //push the remaining string
           amount += 1;
           subs.push(new_sub);
           //print them onto seperate lines
           for (j, sub) in subs.iter().enumerate() {
                
                if i as u16 == global.cur_row {
                    write!(stdout, "{}{}", cursor::Goto(2, y as u16 + 2 + j as u16), sub.blue())?;
                }else {
                    write!(stdout, "{}{}", cursor::Goto(2, y as u16 + 2 + j as u16), sub)?;
               }
           }
           y += amount;
        }else {
            if i as u16 == global.cur_row {
                write!(stdout, "{}{}", cursor::Goto(2, y as u16 + 2), task.name.as_str().blue())?;
            }else {
                write!(stdout, "{}{}", cursor::Goto(2, y as u16 + 2), task.name)?;
            }
            y += 1;
        }
    }

    Ok(())
}

fn print_headings(global: &Global, stdout: &mut Stdout) -> Result<(), io::Error> {
    let step = (global.terminal_w - BORDER) / 4;
    write!(stdout, "{}",cursor::Goto(2, 2))?;
    write!(stdout, "Name")?;

    write!(stdout, "{}", cursor::Goto(step, 2))?;
    write!(stdout, "Created at")?;

    write!(stdout, "{}", cursor::Goto(step * 2, 2))?;
    write!(stdout, "Completed at")?;

    write!(stdout, "{}", cursor::Goto(step * 3, 2))?;
    write!(stdout, "State")?;

    for i in 2..global.terminal_w {
        write!(stdout, "{}┄", cursor::Goto(i, 4))?;
    }

    Ok(())
}

fn print_tui(global: &Global, stdout: &mut Stdout, tasks: &Vec<Task>) -> Result<(), io::Error>{
    //clear the screen
    write!(stdout, "{}", clear::All)?;

    //tui
    draw_grid(global, stdout)?;
    print_headings(global, stdout)?;
    print_tasks(tasks, stdout, global)?;

    //command line
    write!(stdout, "{}{:?}:", cursor::Goto(1, global.terminal_h), global.mode)?; 
    write!(stdout, "{}{}", cursor::Goto(8, global.terminal_h), global.command_line)?;

    stdout.flush()?;

    Ok(())
}

fn parse_key(global: &Global, key: Key) -> Command{
    match global.mode {
        Mode::Normal => {
            match key {
                Key::Char('k') | Key::Up => Command::MoveUp, 
                Key::Char('j') | Key::Down => Command::MoveDown,
                Key::Char('a') => Command::Add,
                Key::Char('x') | Key::Delete => Command::DeleteTask,
                Key::Char('q') | Key::Esc => Command::Quit,
                Key::Char('m') => Command::SwitchState,
                _ => Command::Invalid,
            }
        },
        Mode::Adding => {
           match key {
               Key::Char('\n') => Command::EnterCommand,
               Key::Char(c) => Command::InsertChar(c),
               Key::Backspace => Command::DeleteChar,
               Key::Esc => Command::EnterNormalMode,
               _ => Command::Invalid,
           }
        },
    }
}

fn parse_terminal_command(global: &Global, tasks: &mut Vec<Task>) {
    match global.mode {
        Mode::Adding => {
            let mut date = Local::now().to_string();
            //removes the unecessary time
            date.truncate(16);
            let new_task = Task {
                name: String::from(&global.command_line),
                created_at: date,
                completed_at: String::new(),
                state: State::Todo,
            };
            tasks.push(new_task);
        },
        _ => {},
    }
}

fn read_data() -> Result<Vec<Task>, io::Error> {
    let mut tasks: Vec<Task> = Vec::new();
    let data = fs::read_to_string(DATA_FILE)?;

    if data.trim().is_empty() {
        return Ok(tasks);
    }

    tasks = serde_json::from_str::<Vec<Task>>(&data)?;

    Ok(tasks)
}

fn save_data(tasks: &Vec<Task>) -> Result<(), io::Error> {
    let json = serde_json::to_string(&tasks)?;
    //this also creates a file
    fs::write(DATA_FILE, json)?;
    Ok(())
}

fn main() {
    //enter raw mode
    let mut stdout = stdout().into_raw_mode().unwrap_or_else(|error| {
        eprintln!("Unable to enter raw mode: {error}");
        process::exit(1);
    });

    write!(stdout, "{}", cursor::Hide).unwrap();

    let mut tasks: Vec<Task> = match read_data(){
        Ok(tasks) => tasks,
        Err(e) => {
            eprintln!("Failed to read file: {}, Error: {e}", DATA_FILE);
            process::exit(1);
        }
    };

    //get the terminal size
    let (w, h) = terminal_size().unwrap_or((20, 20));
    let mut global = Global {
        terminal_w: w,
        terminal_h: h,
        command_line: String::new(),
        cur_row: 0,
        start: 0,
        //top border, bottom border, cmdline and heading padding
        end: h - BORDER * 2  - CMDLINE  - HEADING,
        mode: Mode::Normal,
    };

    loop {
        //make it responsive
        global.update_terminal_size();

        if let Err(e) = print_tui(&global, &mut stdout, &tasks) {
            eprintln!("Error drawing the tui: {e}");
            break;
        }
        
        //key event
        let key = match io::stdin().keys().next() {
            Some(key) => {
                match key {
                    Ok(key) => key,
                    Err(_) => continue,
                }
            },
            None => continue,
        };

        let cmd: Command = parse_key(&global, key);

        match cmd {
            Command::Quit => {
                match save_data(&tasks) {
                    Ok(_) => {
                        break;
                    },
                    Err(_) => {
                        global.command_line.clear();
                        global.command_line.push_str("Error saving the data try again");
                    },
                }
            },
            Command::MoveUp => {
                if global.cur_row <= global.start + SCROLL_PADDING && global.start > 0 {
                    global.start -= 1;
                    global.end -= 1;
                    global.cur_row -=1;
                }
                else if global.cur_row > 0 {
                    global.cur_row -= 1;
                }
            },
            Command::MoveDown => {
                if global.cur_row + 1 >= global.end - SCROLL_PADDING && 
                    global.cur_row < tasks.len() as u16 - 1 
                {
                    global.start += 1;
                    global.end += 1;
                    global.cur_row +=1;
                }
                else if global.cur_row + 1 < tasks.len() as u16 {
                    global.cur_row += 1;
                }
            },
            Command::Add => global.mode = Mode::Adding,
            Command::EnterNormalMode => {
                global.command_line.clear();
                global.mode = Mode::Normal;
            },
            Command::InsertChar(c) => global.command_line.push(c),
            Command::EnterCommand => {
                parse_terminal_command(&global, &mut tasks);
                global.command_line.clear();
                global.mode = Mode::Normal;
            },
            Command::DeleteChar => {
                match global.command_line.pop() {
                    Some(c) => c,
                    None => continue,
                };
            },
            Command::DeleteTask => {
                if !tasks.is_empty() {
                tasks.remove(global.cur_row as usize);
                    if global.cur_row > 0 {
                        global.cur_row -=1;
                    }            
                }
            }
            Command::SwitchState => {
                match tasks.get_mut(global.cur_row as usize) {
                    Some(task) => {
                        match task.state {
                            State::Todo => task.state = State::Doing,
                            State::Doing => {
                                task.state = State::Done;
                                let mut cur_date = Local::now().to_string();
                                cur_date.truncate(16);
                                task.completed_at.push_str(&cur_date);
                            },
                            State::Done => {
                                task.completed_at.clear();
                                task.state = State::Todo;
                            }
                        }
                    },
                    None => {},
                }
            }
            _ => {},
        }
    }

    //reset the screen
    write!(stdout, "{}{}{}", clear::All, cursor::Goto(1, 1), cursor::Show).unwrap();
}
