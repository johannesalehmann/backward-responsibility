use std::process::Command;

pub struct PrismRunner {
    path_to_prism: String,
    path_to_java: Option<String>,
}

impl Default for PrismRunner {
    fn default() -> Self {
        Self {
            path_to_prism: "prism".to_string(),
            path_to_java: None,
        }
    }
}

impl PrismRunner {
    pub fn run_prism<I, S>(&self, args: I) -> String
    where
        I: IntoIterator<Item = S>,
        S: AsRef<std::ffi::OsStr>,
    {
        let mut command = self.get_prism_command();
        command.args(args);
        self.execute_prism_command(command)
    }

    pub fn get_prism_command(&self) -> Command {
        let mut command = Command::new(&self.path_to_prism);
        if let Some(path_to_java) = &self.path_to_java {
            command.env("PRISM_JAVA", path_to_java);
        }

        command
    }

    pub fn set_path_to_prism<S: Into<String>>(&mut self, path_to_prism: S) {
        self.path_to_prism = path_to_prism.into();
    }

    pub fn set_path_to_java<S: Into<String>>(&mut self, path_to_java: S) {
        self.path_to_java = Some(path_to_java.into());
    }

    pub fn execute_prism_command(&self, mut command: Command) -> String {
        let output = command
            .output()
            .unwrap_or_else(|err| panic!("Failed to run prism: {}", err));
        if !output.stderr.is_empty() {
            println!("Prism produced an error:");
            match String::from_utf8(output.stderr) {
                Ok(str) => println!("{}", str),
                Err(err) => println!("Prism's stderr is not valid utf8: {}", err),
            }
            panic!("Prism did not run successfully")
        }
        match output.status.success() {
            true => match String::from_utf8(output.stdout) {
                Ok(str) => str,
                Err(err) => panic!("Prism's stdout is not valid utf8: {}", err),
            },
            false => {
                println!(
                    "Prism existed with {}",
                    output
                        .status
                        .code()
                        .map_or("no status code".to_string(), |c| format!(
                            "status code {}",
                            c
                        ))
                );
                match String::from_utf8(output.stdout) {
                    Ok(str) => println!("{}", str),
                    Err(err) => println!("Prism's stdout is not valid utf8: {}", err),
                }
                panic!("Prism did not run successfully")
            }
        }
    }
}
