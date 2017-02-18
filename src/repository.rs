//! defines functions/types related to local repository access.

use std::path::{Path, PathBuf};
use url::Url;
use walkdir::{WalkDir, WalkDirIterator};

use errors::Result;
use remote;
use vcs;


pub struct Repository {
  path: Option<PathBuf>,
  url: Option<Url>,
}

impl Repository {
  /// Make an instance of `Repository` from local path.
  pub fn from_path<P: AsRef<Path>>(path: P) -> Self {
    Repository {
      path: Some(path.as_ref().to_owned()),
      url: None,
    }
  }

  /// Make an instance of `Repository from remote URL.
  pub fn from_url(url: &Url) -> Self {
    Repository {
      path: None,
      url: Some(url.clone()),
    }
  }

  pub fn from_query(query: &str) -> Result<Self> {
    let url = remote::build_url(query)?;
    Ok(Repository::from_url(&url))
  }

  /// Guess the local path from URL and given root directory.
  pub fn guess_path<P: AsRef<Path>>(&mut self, root: P) -> Result<()> {
    let url = self.url.as_ref().ok_or("unknown URL to guess")?;

    let mut path = url.host_str().map(ToOwned::to_owned).ok_or("url.host() is empty")?;
    path += url.path().trim_right_matches(".git");

    self.path = Some(root.as_ref().join(path));

    Ok(())
  }

  /// Perform to clone repository into local path.
  pub fn do_clone(&self, args: &[String], dry_run: bool) -> Result<()> {
    let ref url = self.url.as_ref().ok_or("empty URL")?;
    let ref path = self.path.as_ref().ok_or("empty Path")?;

    if dry_run {
      println!("clone from {:?} into {:?} (args = {:?})",
               url,
               self.path,
               args);
      return Ok(());
    }

    vcs::git::clone(url, path, args)
  }

  pub fn sync_remote_url(&mut self) -> Result<()> {
    if let Some(ref path) = self.path {
      let url = vcs::git::get_upstream_url(path)?;
      debug!("The URL of remote repository is {}", url.as_str());
      self.url = Some(url);
    }
    Ok(())
  }

  pub fn is_already_cloned<P: AsRef<Path>>(&self, root: P) -> bool {
    collect_from(root).into_iter().any(|repo| self.is_same_local(&repo))
  }

  pub fn is_same_local(&self, other: &Self) -> bool {
    self.path
      .as_ref()
      .and_then(|path| other.path.as_ref().map(|o| o.as_path() == path))
      .unwrap_or(false)
  }

  #[cfg(windows)]
  pub fn path_string(&self) -> Option<String> {
    self.path.as_ref().map(|s| s.to_string_lossy().replace("\\", "/"))
  }

  #[cfg(not(windows))]
  pub fn path_string(&self) -> Option<String> {
    self.path.as_ref().map(|s| format!("{}", s.display()))
  }
}

pub fn collect_from<P: AsRef<Path>>(root: P) -> Vec<Repository> {
  WalkDir::new(root.as_ref())
    .follow_links(true)
    .into_iter()
    .filter_entry(|ref entry| {
      if entry.path() == root.as_ref() {
        return true;
      }
      entry.path()
        .parent()
        .map(|path| vcs::detect_from_path(&path).is_none())
        .unwrap_or(true)
    })
    .filter_map(|e| e.ok())
    .filter(|ref entry| vcs::detect_from_path(entry.path()).is_some())
    .map(|entry| Repository::from_path(entry.path()))
    .collect()
}