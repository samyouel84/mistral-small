use std::str::FromStr;

#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    Exit,
    Clear,
    New,
    Message(String),
}

impl FromStr for Command {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();
        Ok(match s.to_lowercase().as_str() {
            "exit" => Command::Exit,
            "clear" => Command::Clear,
            "new" => Command::New,
            _ => Command::Message(s.to_string()),
        })
    }
}

pub const COMMAND_BOX: &str = "\
┌──────────────────────────────────────┐\n\
│          Available Commands          │\n\
├──────────────────────────────────────┤\n\
│    `exit`  - Quit the application    │\n\
├──────────────────────────────────────┤\n\
│    `clear` - Clear the screen        │\n\
├──────────────────────────────────────┤\n\
│    `new`   - Start a new chat        │\n\
└──────────────────────────────────────┘";