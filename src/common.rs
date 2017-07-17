//!
//!
//! Common implementation and struct from original implementation.
//! Currently only to avoid duplicate code between 'full' and 'last'
//!

/// Query expect reply, reply does not (reply can reply to reply)
#[derive(RustcDecodable,RustcEncodable,Debug,Clone,PartialEq,Eq)]
pub enum TunnelState {
  // TODO bitunnel need a ReplyCached with sim_key ??? replace actual replycached by SendOnCache
  /// NoTunnel, same as TunnelMode::NoTunnel (TODO remove TunnelMode::NoTunnel when stable)
  /// TODO redefine to Undefined
  TunnelState,
  /// query with all info to establish route (and reply), no local proxy cache
  QueryOnce,
  /// info are cached, a sym is set 
  QueryCached,
  /// Reply with no additional info for reuse, can be use directly (not cached)
  ReplyOnce,
  /// info are cached, can be use by emitter after QueryCached (same as transfer cached)
  ReplyCached,
 // Error are only for query, as error for reply could not be emit again (dest do not know origin
 // of query (otherwhise use NoRepTunnel and only send Query).
//  QError
  /// same as QError but no route just tcid, and error is send as xor of previous with ours.
  QErrorCached,
//  Query(usize), // nb of query possible TODO for reuse 
//  Reply(usize), // nb of reply possible TODO for reuse
}
impl TunnelState {
  pub fn do_cache(&self) -> bool {
    match self {
      &TunnelState::QueryCached 
      | &TunnelState ::ReplyCached 
      | &TunnelState::QErrorCached => true,
      _ => false,
    }
  }
  pub fn from_cache(&self) -> bool {
    match self {
      &TunnelState ::ReplyCached 
      | &TunnelState::QErrorCached => true,
      _ => false,
    }
  }
 
  pub fn add_sim_key(&self) -> bool {
    match self {
      // add key
      &TunnelState::QueryCached => true,
      _ => false,
    }
  }

}

