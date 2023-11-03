use std::{path::Path, sync::OnceLock};

use futures::{Stream, StreamExt};
use rustix::path::Arg;
use tap::Pipe;
use tokio::fs;

use crate::{
    unit::{
        service::loader::load_service, socket::loader::load_socket, target::loader::load_target,
        Unit, UnitDeps, UnitId,
    },
    Rc,
};

pub(crate) fn str_to_unitids(s: &str) -> Box<[UnitId]> {
    s.split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(UnitId::from)
        .collect()
}

static EMPTYSTR: OnceLock<Rc<str>> = OnceLock::new();
pub(crate) fn empty_str() -> Rc<str> {
    EMPTYSTR.get_or_init(|| ("".into())).clone()
}

static EMPTYDEP: OnceLock<Rc<UnitDeps>> = OnceLock::new();
pub(crate) fn empty_dep() -> Rc<UnitDeps> {
    EMPTYDEP.get_or_init(|| UnitDeps::default().into()).clone()
}

pub(crate) async fn load_units_from_dir(
    path: impl AsRef<Path>,
) -> impl Stream<Item = Rc<dyn Unit + Send + Sync + 'static>> {
    let path = path.as_ref();
    if path.is_dir() {
        let dir = tokio::fs::read_dir(path).await.unwrap();
        let dir = tokio_stream::wrappers::ReadDirStream::new(dir);
        dir.filter_map(|e| async {
            match e {
                Ok(e) => {
                    let path = e.path();
                    if let Some(ext) = path.extension() {
                        let f = fs::read_to_string(&path);
                        match ext.as_str().unwrap() {
                            "target" => Some(Rc::new(f.await.ok()?.pipe_as_ref(load_target)) as _),
                            "service" => {
                                Some(Rc::new(f.await.ok()?.pipe_as_ref(load_service)) as _)
                            }
                            "socket" => Some(Rc::new(f.await.ok()?.pipe_as_ref(load_socket)) as _),
                            _ => None,
                        }
                    } else {
                        None
                    }
                }
                Err(_) => todo!(),
            }
        })
    } else {
        todo!()
    }
}
