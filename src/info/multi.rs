//!
//!
//! Multi reply info as used in old implementation (minus error mgmt TODO)
//! TODO with cached associated trait on info, could merge with error.rs??
use std::marker::PhantomData;
use super::super::{
  BincErr,
  RepInfo,
  Info,
  ReplyProvider,
  SymProvider,
  Peer,
};
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



/// Possible multiple reply handling implementation
/// Cost of an enum, it is mainly for testing,c
#[derive(Serialize,Deserialize,Debug,Clone,PartialEq,Eq)]
pub enum MultipleReplyMode {
  /// do not propagate errors
  NoHandling,
  /// send error to peer with designed mode (case where we can know emitter for dest or other error
  /// handling scheme with possible rendezvous point), TunnelMode is optional and used only for
  /// case where the reply mode differs from the original tunnelmode TODO remove?? (actually use
  /// for norep (public) to set origin for dest)
  KnownDest,
  /// route for error is included, end of payload should be proxyied.
  Route,
  /// route for error is included, end of payload should be proxyied, different route is use
  /// Same as bitunnel from original implementation
  OtherRoute,
  /// Mode for reply payload (use by Route and OtherRoute)
  RouteReply,
  // if route is cached (info in local cache), report error with same route  CachedRoute,
  CachedRoute,
}

/// Error handle info include in frame, also use for reply
/// TODO split as ReplyInfo is split
/// TODO generic not in tunnel
#[derive(Serialize,Deserialize,Debug,Clone)]
pub enum MultipleReplyInfo<A> {
  NoHandling,
  KnownDest(A), // TODO add reply mode ?? TODO address instead of key??
  Route, // route headers are to be read afterward, contains sym key
  /// reply info include in route after content
  RouteReply(Vec<u8>), // route headers are to be read afterward, contains sym key
  CachedRoute(Vec<u8>), // contains symkey for peer shadow
}


// TODO implement Info
// TODO next split it (here it is MultiReply whichi is previous enum impl, purpose of refacto is
// getting rid of those enum (only if needed)
// TODO get TunnelProxy info next_proxy_peer and tunnel id as PeerInfo and not reply info (after full running)
// TODO for perf sake should be an enum (at least with the noreply : except for cache impl those
// ar null (too bug in tunnelshadoww) : the double option is not a good sign too
/*pub struct MultipleReplyInfo<E : ExtWrite, P : Peer, TW : TunnelWriterExt> {
  pub info : MultipleReplyInfo<P>,
  // reply route should be seen as a reply info : used to write the payload -> TODO redesign this
  // TODO not TunnelWriterFull in box to share with last
  pub replyroute : Option<(E,Box<TW>)>,
  //replyroute : Option<Box<(E,TunnelWriterFull<E,P,TW>)>>,
}*/

impl<A : Serialize + DeserializeOwned + Clone + Eq> Info for MultipleReplyInfo<A> {

  #[inline]
  fn do_cache (&self) -> bool {
    match self {
      &MultipleReplyInfo::CachedRoute(_) => true,
      _ => false,
    }
  }



  fn write_in_header<W : Write>(&mut self, inw : &mut W) -> Result<()> {
    bin_encode(inw, self, Infinite).map_err(|e|BincErr(e))?;

//    if self.info.do_cache() { 

    // write tunnel simkey
          //let shadsim = <<P as Peer>::Shadow as Shadow>::new_shadow_sim().unwrap();
//      let mut buf :Vec<u8> = Vec::new();
 //     try!(inw.write_all(&self.replykey.as_ref().unwrap()[..]));
//      try!(self.2.as_mut().unwrap().send_shadow_simkey(&mut inw)); 
/*      let mut cbuf = Cursor::new(buf);
      println!("one");
      try!(self.2.as_mut().unwrap().send_shadow_simkey(&mut cbuf));
 let mut t = cbuf.into_inner();
 println!("{:?}",t);
      inw.write_all(&mut t [..]);
    } else {
      println!("two");*/
  //  }


    Ok(())
  }

  fn write_read_info<W : Write>(&mut self, w : &mut W) -> Result<()> {
    if let &mut MultipleReplyInfo::RouteReply(ref k) = self {
      bin_encode(w,k, Infinite).map_err(|e|BincErr(e))?;
    }
    Ok(())
  }
  fn read_from_header<R : Read>(r : &mut R) -> Result<Self> {
    Ok(bin_decode(r, Infinite).map_err(|e|BincErr(e))?)
  }

  fn read_read_info<R : Read>(&mut self, r : &mut R) -> Result<()> {
    // as dest this is called multiple times and thus we redifine it
    if let &mut MultipleReplyInfo::RouteReply(ref mut k) = self {
       *k = bin_decode(r, Infinite).map_err(|e|BincErr(e))?;
    }
    Ok(())
  }

}

impl<A : Serialize + DeserializeOwned + Clone + Eq> RepInfo for MultipleReplyInfo<A> {
  #[inline]
  fn require_additional_payload(&self) -> bool {
    if let &MultipleReplyInfo::Route = self { true } else {false}
  }


  /// TODO remove called once
  fn get_reply_key(&self) -> Option<&Vec<u8>> {
    match self {
 //     &MultipleReplyInfo::RouteReply(ref k) => Some(k),
      &MultipleReplyInfo::CachedRoute(ref k) => Some(k),
      &MultipleReplyInfo::RouteReply(ref k) => Some(k),
      _ => None,
    }
  }

}

/// TODO E as explicit limiter named trait for readability
pub struct ReplyInfoProvider<P : Peer,SSW,SSR, SP : SymProvider<SSW,SSR>> {
  pub mode : MultipleReplyMode,
  // for different reply route
  pub symprov : SP,
  pub _p : PhantomData<(P,SSW,SSR)>,
}

/// TODO macro inherit??
impl<P:Peer,SSW,SSR,SP : SymProvider<SSW,SSR>> SymProvider<SSW,SSR> for ReplyInfoProvider<P,SSW,SSR,SP> {

  #[inline]
  fn new_sym_key (&mut self) -> Vec<u8> {
    self.symprov.new_sym_key()
  }
  #[inline]
  fn new_sym_writer (&mut self, k : Vec<u8>) -> SSW {
    self.symprov.new_sym_writer(k)
  }
  #[inline]
  fn new_sym_reader (&mut self, k : Vec<u8>) -> SSR {
    self.symprov.new_sym_reader(k)
  }

}

impl<P : Peer,SSW,SSR,SP : SymProvider<SSW,SSR>> ReplyProvider<P, MultipleReplyInfo<<P as Peer>::Address>> for ReplyInfoProvider<P,SSW,SSR,SP> {

  /// Error infos bases for peers
  fn new_reply (&mut self, route : &[&P]) -> Vec<MultipleReplyInfo<<P as Peer>::Address>> {
     let l = route.len();
     match self.mode {
       MultipleReplyMode::NoHandling => vec![MultipleReplyInfo::NoHandling;l-1],
       MultipleReplyMode::KnownDest => {
         let mut res = vec![MultipleReplyInfo::NoHandling;l-1];
         res[l-2] = MultipleReplyInfo::KnownDest(route[0].get_address().clone());
         res
       },
       MultipleReplyMode::OtherRoute => {
         let mut res = vec![MultipleReplyInfo::NoHandling;l-1];
         res[l-2] = MultipleReplyInfo::Route;
         //res[l-2] = MultipleReplyInfo::Route(self.new_sym_key(route[l-1]));
         res
       },
       MultipleReplyMode::Route => {
         let mut res = vec![MultipleReplyInfo::NoHandling;l-1];
         //res[l-2] = MultipleReplyInfo::Route(self.new_sym_key(route[l-1]));
         res[l-2] = MultipleReplyInfo::Route;
         res
       },

       MultipleReplyMode::RouteReply => {
         let mut res : Vec<MultipleReplyInfo<_>> = Vec::with_capacity(l-1);
         for _ in 1..l {
           res.push(MultipleReplyInfo::RouteReply(self.new_sym_key()))
         }
         res
       },
       MultipleReplyMode::CachedRoute => {
         let mut res : Vec<MultipleReplyInfo<_>> = Vec::with_capacity(l-1);
         for _ in 1..l {
           res.push(MultipleReplyInfo::CachedRoute(self.new_sym_key()))
         }
         res
       },
     }
  }
}

/// specific provider for no rpe
pub struct NoMultiRepProvider;

impl<P : Peer> ReplyProvider<P, MultipleReplyInfo<<P as Peer>::Address>> for NoMultiRepProvider {
  #[inline]
  fn new_reply (&mut self, p : &[&P]) -> Vec<MultipleReplyInfo<<P as Peer>::Address>> {
    vec![MultipleReplyInfo::NoHandling;p.len()-1]
  }
}

impl<SSW,SSR> SymProvider<SSW,SSR> for NoMultiRepProvider {
  fn new_sym_key (&mut self) -> Vec<u8> {
    unimplemented!()
  }
  fn new_sym_writer (&mut self, _ : Vec<u8>) -> SSW {
    unimplemented!()
  }
  fn new_sym_reader (&mut self, _ : Vec<u8>) -> SSR {
    unimplemented!()
  }
}

