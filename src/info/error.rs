//! Module containing default implementation.
//! For instance default trait implementation to import in order to have a full tunnel
//! implementation even if the tunnel only implement send only or do not manage error
use rand::os::OsRng;
use rand::Rng;
use super::super::{
  BincErr,
  Info,
  ErrorProvider,
  Peer,
};
/// wrong use need redesign TODO redesign it on specific trait (not TW as param)
use bincode::Infinite;
use bincode::{
  serialize_into as bin_encode, 
  deserialize_from as bin_decode,
};
use std::io::{
  Write,
  Read,
  Result,
};
use serde::{
  Serialize, 
  Deserialize, 
  Serializer, 
  Deserializer,
};
use serde::de::DeserializeOwned;


#[derive(Serialize,Deserialize,Debug,Clone,PartialEq,Eq)]
pub enum MultipleErrorMode {
  /// do not propagate errors
  NoHandling,
  // TODO bitunnel route
  // if route is cached (info in local cache), report error with same route  CachedRoute,
  CachedRoute,
}


/// Only QueryCached error here
/// With multiple reply route, it could also be use in other case (ReplyCached to switch to other
/// route for instance)
#[derive(Serialize,Deserialize,Debug,Clone)]
pub enum MultipleErrorInfo {
  NoHandling,
//  Route(usize), // usize is error code (even if we reply with full route we still consider error code only
  /// only usable with QueryCached state : with single route config it is the only senseful usage.
  /// Please note that it could be use to clear cache
  CachedRoute(usize), // usize is error code
}
impl Info for MultipleErrorInfo {

  #[inline]
  fn do_cache (&self) -> bool {
    if let &MultipleErrorInfo::CachedRoute(..) = self {
      true
    } else {
      false
    }
  }

  fn write_in_header<W : Write>(&mut self, inw : &mut W) -> Result<()> {
    bin_encode(inw, self, Infinite).map_err(|e|BincErr(e))?;
    Ok(())
  }

  #[inline]
  fn write_read_info<W : Write>(&mut self, _ : &mut W) -> Result<()> {
    Ok(())
  }

  fn read_from_header<R : Read>(r : &mut R) -> Result<Self> {
    Ok(bin_decode(r, Infinite).map_err(|e|BincErr(e))?)
  }

  #[inline]
  fn read_read_info<R : Read>(&mut self, _ : &mut R) -> Result<()> {
    Ok(())
  }

}

/// TODO E as explicit limiter named trait for readability
pub struct MulErrorProvider {
  mode : MultipleErrorMode,
  gen : OsRng,
}


//type P : Peer;
  //type TW : TunnelWriter;
  //type TR : TunnelReader;


impl MulErrorProvider {
  pub fn new (mode : MultipleErrorMode) -> Result<MulErrorProvider> {
    Ok(MulErrorProvider {
      mode : mode,
      gen : OsRng::new()?,
    })
  }
}
impl<P : Peer> ErrorProvider<P, MultipleErrorInfo> for MulErrorProvider {
  /// Error infos bases for peers
  fn new_error_route (&mut self, route : &[&P]) -> Vec<MultipleErrorInfo> {

    let l = route.len();
     match self.mode {
       MultipleErrorMode::NoHandling => 
         vec![MultipleErrorInfo::NoHandling;l-1],
       MultipleErrorMode::CachedRoute => {
         let mut res : Vec<MultipleErrorInfo> = Vec::with_capacity(l-1);
         for _ in 1..l {
           let mut errorid = self.gen.gen();
           while errorid == 0 {
              errorid = self.gen.gen();
           }
           res.push(MultipleErrorInfo::CachedRoute(errorid))
         }
         res
       },
     }
 
  }
}

/// specific provider for no error
pub struct NoErrorProvider;

impl<P : Peer> ErrorProvider<P, MultipleErrorInfo> for NoErrorProvider {
  fn new_error_route (&mut self, p : &[&P]) -> Vec<MultipleErrorInfo> {
    vec![MultipleErrorInfo::NoHandling;p.len()-1]
/*    let mut r = Vec::with_capacity(p.len());
    for _ in 0..p.len() {
      r.push(
         MultipleErrorInfo::NoHandling
      );
    }
    r*/
  }
}

