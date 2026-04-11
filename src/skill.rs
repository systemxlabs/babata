use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::{BabataResult, error::BabataError, utils::babata_dir};

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SkillFrontmatter {
    pub name: String,
    pub description: String,
}

/// Get the skill directory path for a given skill name
fn skill_dir(name: &str) -> BabataResult<PathBuf> {
    Ok(babata_dir()?.join("skills").join(name))
}

/// Get the SKILL.md file path for a given skill name
fn skill_file_path(name: &str) -> BabataResult<PathBuf> {
    Ok(skill_dir(name)?.join("SKILL.md"))
}

/// Check if a skill exists by name
pub fn skill_exists(name: &str) -> BabataResult<bool> {
    let path = skill_file_path(name)?;
    Ok(path.exists())
}

/// Delete a skill by name
pub fn delete_skill(name: &str) -> BabataResult<()> {
    let dir = skill_dir(name)?;
    if dir.exists() {
        std::fs::remove_dir_all(&dir).map_err(|err| {
            BabataError::internal(format!(
                "Failed to delete skill directory '{}': {}",
                dir.display(),
                err
            ))
        })?;
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq)]
pub struct Skill {
    pub path: PathBuf,
    pub frontmatter: SkillFrontmatter,
    pub body: String,
}

pub fn load_skills() -> BabataResult<Vec<Skill>> {
    let dir = babata_dir()?.join("skills");
    load_skills_from_dir(&dir)
}

fn load_skills_from_dir(dir: &Path) -> BabataResult<Vec<Skill>> {
    if !dir.exists() {
        return Ok(Vec::new());
    }

    if !dir.is_dir() {
        return Err(BabataError::config(format!(
            "Skills path '{}' is not a directory",
            dir.display()
        )));
    }

    let mut skills = Vec::new();
    let entries = std::fs::read_dir(dir).map_err(|err| {
        BabataError::config(format!(
            "Failed to read skills directory '{}': {}",
            dir.display(),
            err
        ))
    })?;

    for entry in entries {
        let entry = entry.map_err(|err| {
            BabataError::config(format!(
                "Failed to read skills directory entry in '{}': {}",
                dir.display(),
                err
            ))
        })?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let skill_path = path.join("SKILL.md");
        if !skill_path.is_file() {
            continue;
        }

        let content = std::fs::read_to_string(&skill_path).map_err(|err| {
            BabataError::config(format!(
                "Failed to read skill file '{}': {}",
                skill_path.display(),
                err
            ))
        })?;
        let (frontmatter, body) = parse_skill_content(&content, &skill_path)?;
        skills.push(Skill {
            path: skill_path,
            frontmatter,
            body,
        });
    }

    skills.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(skills)
}

fn parse_skill_content(content: &str, path: &Path) -> BabataResult<(SkillFrontmatter, String)> {
    let mut lines = content.lines();
    let Some(first) = lines.next() else {
        return Err(BabataError::config(format!(
            "Skill file '{}' is empty or missing headers",
            path.display()
        )));
    };
    if first != "---" {
        return Err(BabataError::config(format!(
            "Skill file '{}' is missing yaml headers (expected starting '---')",
            path.display()
        )));
    }

    let mut header_lines = Vec::new();
    let mut body_lines = Vec::new();
    let mut in_headers = true;

    for line in lines {
        if in_headers {
            if line == "---" {
                in_headers = false;
                continue;
            }
            header_lines.push(line);
        } else {
            body_lines.push(line);
        }
    }

    if in_headers {
        return Err(BabataError::config(format!(
            "Skill file '{}' starts with '---' but has no closing '---'",
            path.display()
        )));
    }

    let header_raw = header_lines.join("\n");
    let body = body_lines.join("\n");
    let headers = serde_yaml::from_str::<SkillFrontmatter>(&header_raw).map_err(|err| {
        BabataError::config(format!(
            "Failed to parse skill headers in '{}': {}",
            path.display(),
            err
        ))
    })?;

    Ok((headers, body))
}
