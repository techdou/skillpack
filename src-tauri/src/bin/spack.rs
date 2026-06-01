//! SkillPack CLI - `spack` command-line interface
//!
//! Usage:
//!   spack install <git-url> --name <name> [--skill-root <path>]
//!   spack list [--pack <name>]
//!   spack remove <name>
//!   spack update [--name <name>]
//!   spack link <skill-name> --project <path> --pack <name> [--target codex|agents|claude|cursor]
//!   spack unlink <skill-name> --project <path>
//!   spack project add <path>
//!   spack project remove <path>
//!   spack project list
//!   spack plugin list
//!   spack plugin toggle <key> <on|off>

use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        print_usage();
        std::process::exit(1);
    }

    let result = run(&args);
    if let Err(e) = result {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

fn run(args: &[String]) -> Result<(), String> {
    match args[1].as_str() {
        "install" => cmd_install(args),
        "list" => cmd_list(args),
        "remove" => cmd_remove(args),
        "update" => cmd_update(args),
        "link" => cmd_link(args),
        "unlink" => cmd_unlink(args),
        "project" => cmd_project(args),
        "plugin" => cmd_plugin(args),
        "help" | "--help" | "-h" => {
            print_usage();
            Ok(())
        }
        _ => Err(format!("Unknown command: {}", args[1])),
    }
}

fn get_arg<'a>(args: &'a [String], flag: &str) -> Option<&'a str> {
    args.iter()
        .position(|a| a == flag)
        .and_then(|i| args.get(i + 1).map(|s| s.as_str()))
}

fn get_pos_arg<'a>(args: &'a [String], after_subcmd: usize) -> Option<&'a str> {
    // Find first positional arg after the subcommand and flags
    let subcmd_end = 2 + after_subcmd;
    if args.len() > subcmd_end {
        for i in subcmd_end..args.len() {
            if !args[i].starts_with('-') {
                return Some(&args[i]);
            }
        }
    }
    None
}

fn print_usage() {
    println!("SkillPack v1.0.0 - AI Coding Skills Package Manager");
    println!();
    println!("Commands:");
    println!("  install <url> --name <name> [--skill-root <path>]  Install a skill pack");
    println!(
        "  list [--pack <name>]                                List packs or skills in a pack"
    );
    println!("  remove <name>                                       Remove a pack");
    println!("  update [--name <name>]                              Update pack(s)");
    println!("  link <skill> --project <path> --pack <name> [--target <chain>]");
    println!("                                                      Link skill to project");
    println!("  unlink <skill> --project <path>                     Unlink skill from project");
    println!("  project add <path>                                  Register a project");
    println!("  project remove <path>                               Unregister a project");
    println!("  project list                                        List projects");
    println!("  plugin list                                         List Codex plugins");
    println!("  plugin toggle <key> <on|off>                        Toggle Codex plugin");
    println!();
    println!("Targets: codex, agents, claude, cursor");
}

// --- Command implementations ---

fn cmd_install(args: &[String]) -> Result<(), String> {
    let url = get_pos_arg(args, 0).ok_or("Usage: spack install <url> --name <name>")?;
    let name = get_arg(args, "--name").ok_or("Missing --name")?.to_string();
    let skill_root = get_arg(args, "--skill-root").map(|s| s.to_string());

    println!("Installing pack '{}' from {}...", name, url);
    let pack = skillpack_core::pack_install(url.to_string(), name, skill_root)?;
    println!("Installed {} skills:", pack.skills.len());
    for s in &pack.skills {
        println!("  - {}", s);
    }
    Ok(())
}

fn cmd_list(args: &[String]) -> Result<(), String> {
    let packs = skillpack_core::pack_list()?;
    if let Some(pack_name) = get_arg(args, "--pack") {
        let (_, pack) = packs
            .into_iter()
            .find(|(n, _)| n == pack_name)
            .ok_or_else(|| format!("Pack '{}' not found", pack_name))?;
        println!("Skills in {} ({}):", pack_name, pack.skills.len());
        for s in &pack.skills {
            println!("  {}", s);
        }
    } else {
        if packs.is_empty() {
            println!("No packs installed.");
        } else {
            println!("{:<20} {:<10} {}", "PACK", "SKILLS", "SOURCE");
            for (name, pack) in &packs {
                println!("{:<20} {:<10} {}", name, pack.skills.len(), pack.source);
            }
        }
    }
    Ok(())
}

fn cmd_remove(args: &[String]) -> Result<(), String> {
    let name = get_pos_arg(args, 0).ok_or("Usage: spack remove <name>")?;
    println!("Removing pack '{}'...", name);
    skillpack_core::pack_remove(name.to_string())?;
    println!("Removed.");
    Ok(())
}

fn cmd_update(args: &[String]) -> Result<(), String> {
    let name = get_arg(args, "--name").map(|s| s.to_string());
    println!("Updating...");
    let updated = skillpack_core::pack_update(name)?;
    if updated.is_empty() {
        println!("Nothing to update.");
    } else {
        for n in &updated {
            println!("  Updated: {}", n);
        }
    }
    Ok(())
}

fn cmd_link(args: &[String]) -> Result<(), String> {
    let skill_name =
        get_pos_arg(args, 0).ok_or("Usage: spack link <skill> --project <path> --pack <name>")?;
    let project = get_arg(args, "--project")
        .ok_or("Missing --project")?
        .to_string();
    let pack = get_arg(args, "--pack").ok_or("Missing --pack")?.to_string();
    let target = get_arg(args, "--target").unwrap_or("codex").to_string();

    println!(
        "Linking {} from {} to {} ({})...",
        skill_name, pack, project, target
    );
    skillpack_core::skill_link(project, skill_name.to_string(), pack, target)?;
    println!("Linked.");
    Ok(())
}

fn cmd_unlink(args: &[String]) -> Result<(), String> {
    let skill_name = get_pos_arg(args, 0).ok_or("Usage: spack unlink <skill> --project <path>")?;
    let project = get_arg(args, "--project")
        .ok_or("Missing --project")?
        .to_string();

    println!("Unlinking {} from {}...", skill_name, project);
    skillpack_core::skill_unlink(project, skill_name.to_string())?;
    println!("Unlinked.");
    Ok(())
}

fn cmd_project(args: &[String]) -> Result<(), String> {
    let subcmd = args.get(2).map(|s| s.as_str()).unwrap_or("");
    match subcmd {
        "add" => {
            let path = args.get(3).ok_or("Usage: spack project add <path>")?;
            let proj = skillpack_core::project_add(path.to_string())?;
            println!("Added project: {} ({})", proj.name, proj.path);
            Ok(())
        }
        "remove" => {
            let path = args.get(3).ok_or("Usage: spack project remove <path>")?;
            skillpack_core::project_remove(path.to_string())?;
            println!("Removed project: {}", path);
            Ok(())
        }
        "list" => {
            let projects = skillpack_core::project_list()?;
            if projects.is_empty() {
                println!("No projects registered.");
            } else {
                println!("{:<30} {:<10} {}", "NAME", "LINKED", "PATH");
                for p in &projects {
                    println!("{:<30} {:<10} {}", p.name, p.linked_skills_count, p.path);
                }
            }
            Ok(())
        }
        _ => Err("Usage: spack project <add|remove|list>".into()),
    }
}

fn cmd_plugin(args: &[String]) -> Result<(), String> {
    let subcmd = args.get(2).map(|s| s.as_str()).unwrap_or("");
    match subcmd {
        "list" => {
            let plugins = skillpack_core::plugin_list()?;
            if plugins.is_empty() {
                println!("No Codex plugins found.");
            } else {
                println!("{:<40} {:<8} {}", "PLUGIN", "STATUS", "SOURCE");
                for p in &plugins {
                    let status = if p.enabled { "ON" } else { "OFF" };
                    println!("{:<40} {:<8} {}", p.key, status, p.source);
                }
            }
            Ok(())
        }
        "toggle" => {
            let key = args
                .get(3)
                .ok_or("Usage: spack plugin toggle <key> <on|off>")?;
            let state = args.get(4).ok_or("Missing on/off")?;
            let enabled = match state.to_lowercase().as_str() {
                "on" | "true" | "1" => true,
                "off" | "false" | "0" => false,
                _ => return Err("State must be on/off".into()),
            };
            skillpack_core::plugin_toggle(key.to_string(), enabled)?;
            println!(
                "Plugin {} set to {}.",
                key,
                if enabled { "ON" } else { "OFF" }
            );
            println!("Restart Codex for changes to take effect.");
            Ok(())
        }
        _ => Err("Usage: spack plugin <list|toggle>".into()),
    }
}
