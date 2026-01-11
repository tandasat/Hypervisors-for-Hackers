use std::{
    fmt,
    fs::{self, File},
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
    process::Command,
    str::FromStr,
    sync::mpsc::channel,
    thread,
    time::Duration,
};

use anyhow::{Ok, Result, ensure};
use colored::Colorize;

use crate::{
    Profile,
    config::{LOG_PATH, MODULE_NAME, PASSWORD, SNAPSHOT_NAME, USER_NAME, VMX_PATH},
    workspace_root_dir,
};

pub(crate) fn run(profile: Profile) -> Result<()> {
    let vmx_path = VmxFile::new(VMX_PATH.into());

    // Shutdown the VM if it is running.
    vmrun(
        vmx_path.clone(),
        VmRunCommand::Stop(PowerControl::Force),
        IgnoreError::Yes,
    )?;

    // Then, close the VMware Workstation window. If the window remains open,
    // the VM does not start after reverting a snapshot.
    let _unused = Command::new("taskkill")
        .args(["/f", "/t", "/im", "vmware.exe"])
        .output()?;

    // Delete the log file before starting the VM.
    if Path::new(LOG_PATH).exists() {
        fs::remove_file(LOG_PATH)?;
    }

    // Start the VM and show logs using threads.
    let _unused = thread::Builder::new()
        .name("vmrun".to_owned())
        .spawn(move || vmrun_thread(profile));
    let _unused = thread::Builder::new()
        .name("logging".to_owned())
        .spawn(log_thread);

    // Finally, indefinitely run the VM until CTRL+C is pressed.
    let (tx, rx) = channel();
    ctrlc::set_handler(move || tx.send(()).unwrap())?;
    rx.recv()?;

    println!("🕒 Shutting down the VM");
    vmrun(
        vmx_path,
        VmRunCommand::Stop(PowerControl::Force),
        IgnoreError::Yes,
    )
}

fn vmrun_thread(profile: Profile) {
    fn run_commands(profile: Profile) -> Result<()> {
        const SC_PATH: &str = r"C:\Windows\System32\sc.exe";
        const SERVICE_NAME: &str = MODULE_NAME;

        let guest_path = r"C:\Users\".to_owned() + USER_NAME + r"\Desktop\" + MODULE_NAME + ".sys";
        let host_path = workspace_root_dir()
            .join("target")
            .join(profile.to_string())
            .join(MODULE_NAME.to_owned() + "_package")
            .join(MODULE_NAME.to_owned() + ".sys");
        let vmx_path = VmxFile::new(VMX_PATH.into());
        let cred = Credential::new(USER_NAME.to_owned(), PASSWORD.to_owned());

        println!("🕒 Reverting the snapshot: {SNAPSHOT_NAME}");
        vmrun(
            vmx_path.clone(),
            VmRunCommand::RevertToSnapshot(SNAPSHOT_NAME.to_owned()),
            IgnoreError::No,
        )?;

        println!("🕒 Starting the VM (press CTRL+C to terminate it)");
        vmrun(
            vmx_path.clone(),
            VmRunCommand::Start(Gui::Show),
            IgnoreError::No,
        )?;

        println!("🕒 Deleting an old driver file in the VM");
        vmrun(
            vmx_path.clone(),
            VmRunCommand::DeleteFileInGuest(
                cred.clone(),
                GuestPath::new(PathBuf::from_str(&guest_path)?),
            ),
            IgnoreError::Yes,
        )?;

        println!("🕒 Copying the new driver file to the VM");
        vmrun(
            vmx_path.clone(),
            VmRunCommand::CopyFileFromHostToGuest(
                cred.clone(),
                host_path,
                GuestPath::new(PathBuf::from_str(&guest_path)?),
            ),
            IgnoreError::No,
        )?;

        println!("🕒 Creating the '{SERVICE_NAME}' service in the VM");
        vmrun(
            vmx_path.clone(),
            VmRunCommand::RunProgramInGuest(
                cred.clone(),
                GuestPath::new(PathBuf::from_str(SC_PATH)?),
                vec![
                    "create".to_owned(),
                    SERVICE_NAME.to_owned(),
                    "type=".to_owned(),
                    "kernel".to_owned(),
                    "binPath=".to_owned(),
                    guest_path,
                ],
            ),
            IgnoreError::No,
        )?;

        println!("🕒 Starting the driver in the VM");
        vmrun(
            vmx_path,
            VmRunCommand::RunProgramInGuest(
                cred,
                GuestPath::new(PathBuf::from_str(SC_PATH)?),
                vec!["start".to_owned(), SERVICE_NAME.to_owned()],
            ),
            IgnoreError::No,
        )
    }

    run_commands(profile).expect("vmrun should run all commands");
}

fn log_thread() {
    fn wait_and_show_logs() -> Result<()> {
        while !Path::new(LOG_PATH).exists() {
            thread::sleep(Duration::from_millis(100));
        }

        let file = File::open(LOG_PATH)?;
        let mut reader = BufReader::new(&file);
        loop {
            let mut line = String::new();
            let bytes_read = reader.read_line(&mut line)?;
            if bytes_read > 0 {
                // Show the log line with a color if needed.
                if line.contains(":ERROR:") {
                    print!("{}", line.red());
                } else if line.contains(":WARN :") {
                    print!("{}", line.yellow());
                } else {
                    print!("{line}");
                }
            } else {
                thread::sleep(Duration::from_millis(100));
            }
        }
    }

    wait_and_show_logs().expect("boo!");
}

fn vmrun(vmx_path: VmxFile, command: VmRunCommand, error_handling: IgnoreError) -> Result<()> {
    const VMRUN: &str = r"C:\Program Files (x86)\VMware\VMware Workstation\vmrun.exe";
    const VM_PASSWORD: &str = "12345678";

    let vmx_path = vmx_path.0.into_os_string().into_string().unwrap();
    let mut vmrun = Command::new(VMRUN);
    let process = match command {
        VmRunCommand::RevertToSnapshot(snapshot_name) => vmrun.args([
            "-T",
            "ws",
            "-vp",
            VM_PASSWORD,
            "revertToSnapshot",
            &vmx_path,
            &snapshot_name,
        ]),
        VmRunCommand::Start(gui) => vmrun.args([
            "-T",
            "ws",
            "-vp",
            VM_PASSWORD,
            "start",
            &vmx_path,
            &gui.to_string(),
        ]),
        VmRunCommand::Stop(power) => vmrun.args([
            "-T",
            "ws",
            "-vp",
            VM_PASSWORD,
            "stop",
            &vmx_path,
            &power.to_string(),
        ]),
        VmRunCommand::DeleteFileInGuest(cred, file_path) => {
            let file_path = file_path.0.into_os_string().into_string().unwrap();
            vmrun.args([
                "-T",
                "ws",
                "-vp",
                VM_PASSWORD,
                "-gu",
                &cred.user,
                "-gp",
                &cred.pass,
                "deleteFileInGuest",
                &vmx_path,
                &file_path,
            ])
        }
        VmRunCommand::CopyFileFromHostToGuest(cred, src_path, dst_path) => {
            let src_path = src_path.into_os_string().into_string().unwrap();
            let dst_path = dst_path.0.into_os_string().into_string().unwrap();
            vmrun.args([
                "-T",
                "ws",
                "-vp",
                VM_PASSWORD,
                "-gu",
                &cred.user,
                "-gp",
                &cred.pass,
                "copyFileFromHostToGuest",
                &vmx_path,
                &src_path,
                &dst_path,
            ])
        }
        VmRunCommand::RunProgramInGuest(cred, program_path, args) => {
            let program_path = program_path.0.into_os_string().into_string().unwrap();
            let mut all_args = vec![
                "-T",
                "ws",
                "-vp",
                VM_PASSWORD,
                "-gu",
                &cred.user,
                "-gp",
                &cred.pass,
                "runProgramInGuest",
                &vmx_path,
                &program_path,
            ];
            all_args.extend(&args.iter().map(String::as_str).collect::<Vec<&str>>());
            vmrun.args(all_args)
        }
    };

    match error_handling {
        IgnoreError::Yes => {
            let _unused = process.output()?;
        }
        IgnoreError::No => {
            let status = process.spawn()?.wait()?;
            let args: Vec<_> = vmrun.get_args().collect();
            ensure!(
                status.success(),
                format!("vmrun {args:?} failed with {status:?}")
            );
        }
    }
    Ok(())
}

#[derive(Clone, Debug)]
enum VmRunCommand {
    Start(Gui),
    Stop(PowerControl),
    RevertToSnapshot(String),
    DeleteFileInGuest(Credential, GuestPath),
    CopyFileFromHostToGuest(Credential, PathBuf, GuestPath),
    RunProgramInGuest(Credential, GuestPath, Vec<String>),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Gui {
    Show,
    #[expect(dead_code)]
    None,
}

impl fmt::Display for Gui {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Gui::Show => write!(f, "gui"),
            Gui::None => write!(f, "nogui"),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PowerControl {
    #[expect(dead_code)]
    Normal,
    Force,
}

impl fmt::Display for PowerControl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PowerControl::Normal => write!(f, "soft"),
            PowerControl::Force => write!(f, "hard"),
        }
    }
}

#[derive(Clone, Debug)]
struct Credential {
    user: String,
    pass: String,
}

impl Credential {
    fn new(user: String, pass: String) -> Self {
        Self { user, pass }
    }
}

#[derive(Clone, Debug)]
struct VmxFile(PathBuf);

impl VmxFile {
    fn new(path: PathBuf) -> Self {
        Self(path)
    }
}

#[derive(Clone, Debug)]
struct GuestPath(PathBuf);

impl GuestPath {
    fn new(path: PathBuf) -> Self {
        Self(path)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum IgnoreError {
    Yes,
    No,
}
