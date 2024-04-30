use std::cmp::Ordering;
use std::hash::{DefaultHasher, Hash, Hasher};
use git2::Repository;

use crate::util::DynResult;

pub fn fetch_or_clone(repo_link: String) -> DynResult<Repository> {
    // the repo_link is hashed and the repo is cloned into ./repos/<hash> rather than ./repos/<repo-name>
    // this is because different repos may have the same name
    let mut hasher = DefaultHasher::new();
    repo_link.hash(&mut hasher);
    let hash = hasher.finish();
    let repo_path = format!("./repos/{hash}");
    let repo_path = std::path::Path::new(&repo_path);

    if repo_path.exists() {
        let repo = Repository::open(repo_path)?;
        let mut remote = repo.find_remote("origin")?;
        remote.fetch::<&str>(&[], None, None)?;         // fetch from origin
    } else {
        Repository::clone(&repo_link, repo_path)?;
    }

    return Ok(Repository::open(repo_path)?);
}

pub fn read_commit_messages(repo: &mut Repository, release_tag: &str, prev_release_tag: &str) -> DynResult<Vec<String>> {
    let release_tag = repo.resolve_reference_from_short_name(release_tag.trim())?;
    let prev_release_tag = repo.resolve_reference_from_short_name(prev_release_tag.trim())?;

    let release_commit = release_tag.peel_to_commit()?;
    let prev_release_commit = prev_release_tag.peel_to_commit()?;
    if prev_release_commit.time().cmp(&release_commit.time()) != Ordering::Less {
        return Err(format!("prev_release_tag doesn't predate release_tag.").into());
    }

    // a revwalk denotes an iterator over commits
    let mut revwalk = repo.revwalk()?;
    revwalk.push(release_commit.id())?;         // initial feature commit

    let mut commit_messages: Vec<String> = vec![];
    for commit_oid in revwalk {
        let commit = repo.find_commit(commit_oid?)?;

        if commit.id() == prev_release_commit.id() {
            return Ok(commit_messages);
        }

        if let Some(message) = commit.message() {       // Commit::message will return None if the message is not valid utf-8
            commit_messages.push(message.to_string());
        }
    }

    return Err(format!("prev_release_tag doesn't precede release_tag.").into());
}