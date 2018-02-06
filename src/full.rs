
use rand::OsRng;
use rand::Rng;
use readwrite_comp::{
  MultiWExt,
  MultiRExt,
  ExtRead,
  ExtWrite,
  CompR,
  CompExtW,
  CompExtWInner,
  CompExtR,
  CompExtRInner,
  DefaultID,
};
use std::io::{
  Write,
  Read,
  Cursor,
  Result,
  Error as IoError,
  ErrorKind as IoErrorKind,
};
use bincode::Infinite;
use bincode::{
  serialize_into as bin_encode, 
  deserialize_from as bin_decode,
};
use super::{
  Peer,
  TunnelReader,
  TunnelReadProv,
  TunnelNoRepReadProv,
  TunnelReaderError,
  TunnelReaderNoRep,
  TunnelWriterExt,
  TunnelReaderExt,
  TunnelError,
  TunnelErrorWriter,
  Info,
  RepInfo,
  BincErr,
  BorrMutErr,
  TunnelNoRep,
  Tunnel,
  TunnelManager,
  TunnelCache,
  TunnelCacheErr,
  SymProvider,
  ErrorProvider,
  ReplyProvider,
  RouteProvider,
  CacheIdProducer,
  TunnelManagerError,
};
use super::common::{
  TunnelState,
};
use super::info::multi::{
  MultipleReplyMode,
  MultipleReplyInfo,
};
use super::info::error::{
  MultipleErrorInfo,
  MultipleErrorMode,
};
use std::marker::PhantomData;











/// Generic Tunnel Traits, use as a traits container for a generic tunnel implementation
/// (to reduce number of trait parameter), it is mainly here to reduce number of visible trait
/// parameters in code, 
pub trait GenTunnelTraits {
  type P : Peer;
  /// Reply frame limiter (specific to use of reply once with header in frame
  type LW : ExtWrite + Clone; // limiter
  type LR : ExtRead + Clone; // limiter
  type SSW : ExtWrite;// symetric writer
  type SSR : ExtRead;// seems userless (in rpely provider if needed)
  type TC : TunnelCache<(TunnelCachedWriterExt<Self::SSW,Self::LW>,<Self::P as Peer>::Address),MultiRExt<Self::SSR>>
    + TunnelCacheErr<(ErrorWriter,<Self::P as Peer>::Address), MultipleErrorInfo> + CacheIdProducer;
  type EW : TunnelErrorWriter;
  type RP : RouteProvider<Self::P>;
  /// Reply writer use only to include a reply envelope
  type RW : TunnelWriterExt;
  type REP : ReplyProvider<Self::P, MultipleReplyInfo<<Self::P as Peer>::Address>>;
  type SP : SymProvider<Self::SSW,Self::SSR> + Clone;
  type TNR : TunnelNoRep<P=Self::P,W=Self::RW>;
  type EP : ErrorProvider<Self::P, MultipleErrorInfo>;
}

/// Reply and Error info for full are currently hardcoded MultipleReplyInfo
/// This is not the cas for FullW or FullR, that way only Full need to be reimplemented to use less 
/// generic mode TODO make it generic ? (associated InfoMode type and info constructor)
/// TODO remove E ?? or put in GenTunnel (not really generic
/// TODO multiplereplymode generic
pub struct Full<TT : GenTunnelTraits> {
  pub me : TT::P,
  pub reply_mode : MultipleReplyMode,
  pub error_mode : MultipleErrorMode,
  pub cache : TT::TC,
//  pub sym_prov : TT::SP,
  pub route_prov : TT::RP,
  pub reply_prov : TT::REP,
  pub sym_prov : TT::SP,
  pub tunrep : TT::TNR,
  pub error_prov : TT::EP,
  pub rng : OsRng,
  pub limiter_proto_w : TT::LW,
  pub limiter_proto_r : TT::LR,
  pub reply_once_buf_size : usize,
  pub _p : PhantomData<TT>,
}
pub struct FullReadProv<TT : GenTunnelTraits> {
  pub me : TT::P,
  pub limiter_proto_r : TT::LR,
  pub limiter_proto_w : TT::LW,
  pub sym_prov : TT::SP,
  pub reply_once_buf_size : usize,
  pub _p : PhantomData<TT>,
}


type Shadows<P,RI,EI,LW,TW> = CompExtW<MultiWExt<TunnelShadowW<P,RI,EI,LW,TW>>,LW>;
//type Shadows<P : Peer, RI : Info, EI : Info,LW : ExtWrite,TW : TunnelWriterExt> = CompExtW<MultiWExt<TunnelShadowW<P,RI,EI,LW,TW>>,LW>;

/**
 * No impl for instance when no error or no reply
 *
 *
 * Full writer : use for all write and reply, could be split but for now I use old code.
 *
 */
pub struct FullW<RI : RepInfo, EI : Info, P : Peer, LW : ExtWrite,TW : TunnelWriterExt > {
  state: TunnelState,
  /// Warning if set it means cache is use, it is incompatible with a non cache tunnel state
  /// (reading will fail if state is not right, so beware when creating FullW)
  current_cache_id: Option<Vec<u8>>,
  shads: Shadows<P,RI,EI,LW,TW>,
}

/**
 * reply to both fullw or proxy headers and act as a single symetric of TunnelShadowW
 *
 * Similar to old TunnelShadowExt impl.
 */
pub struct FullR<RI : RepInfo, EI : Info, P : Peer, LR : ExtRead> {
  state: TunnelState,
  // TODO try moving in proxy or dest read new method (may be the last info written so no need to be here)
  current_cache_id: Option<Vec<u8>>,
  current_reply_info: Option<RI>,
  current_error_info: Option<EI>,
  next_proxy_peer : Option<<P as Peer>::Address>,
  read_cache : bool, // if cache has been read
  tunnel_id : Option<usize>, // TODO useless remove??
  shad : CompExtR<<P as Peer>::ShadRead,LR>,
  content_limiter : LR,
  need_content_limiter : bool,
  error_code : Option<usize>,
// TODO should be remove when TunnelReader Trait refactor or removed (code for reading will move in new dest
// reader init from tunnel ) : TODO remove !!! put in dest full kind multi while reading or befor
// caching Also true for cacheid. -> require change of trait : init of dest reader with Read as
// param
}

/// keep reference to reader while destreader or proxying
pub struct ProxyFull<OR : ExtRead,SW : ExtWrite, E : ExtWrite, LR : ExtRead> {
  pub origin_read : OR,
  pub kind : ProxyFullKind<SW,E,LR>
}
impl<OR : ExtRead,SW : ExtWrite, E : ExtWrite, LR : ExtRead> TunnelReaderExt for ProxyFull<OR,SW,E,LR> {
  type TR = OR; 
  fn get_reader(self) -> Self::TR {
    self.origin_read
  }
}

impl<OR : ExtRead,SW : ExtWrite, E : ExtWrite, LR : ExtRead> ExtRead for ProxyFull<OR,SW,E,LR> {
  fn read_header<R : Read>(&mut self, _ : &mut R) -> Result<()> {
    // actually already called
    Ok(())
  }

  fn read_from<R : Read>(&mut self, r : &mut R, buf : &mut[u8]) -> Result<usize> {
    match self.kind {
      ProxyFullKind::ReplyCached(_, ref mut rs) => rs.read_from(r,buf),
      ProxyFullKind::QueryOnce(_) | ProxyFullKind::QueryCached(_,_) => self.origin_read.read_from(r,buf),
      ProxyFullKind::ReplyOnce(ref mut limr,ref mut b,_,_,_) if *b => {
        let mut cr = self.origin_read.chain(limr);
        let i = cr.read_from(r,buf)?;
        if cr.in_second() {
          *b = false;
        }
        Ok(i)
      },
      ProxyFullKind::ReplyOnce(ref mut limr,_,_,_,_) => limr.read_from(r,buf),
    }
  }

  fn read_exact_from<R : Read>(&mut self, r : &mut R, buf: &mut[u8]) -> Result<()> {
    match self.kind {
      ProxyFullKind::ReplyCached(_, ref mut rs) => rs.read_exact_from(r, buf),
      ProxyFullKind::QueryOnce(_) | ProxyFullKind::QueryCached(_,_) => self.origin_read.read_exact_from(r,buf),
      ProxyFullKind::ReplyOnce(ref mut limr,ref mut b,_,_,_) if *b => {
        let mut cr = self.origin_read.chain(limr);
        let i = cr.read_exact_from(r,buf)?;
        if cr.in_second() {
          *b = false;
        }
        Ok(i)
      },
      ProxyFullKind::ReplyOnce(ref mut limr,_,_,_,_) => limr.read_exact_from(r,buf),
    }
  }

  fn read_end<R : Read>(&mut self, r : &mut R) -> Result<()> {
    match self.kind {
      ProxyFullKind::ReplyCached(_,ref mut rs) => rs.read_end(r),
      ProxyFullKind::QueryOnce(_) | ProxyFullKind::QueryCached(_,_) => self.origin_read.read_end(r),
      ProxyFullKind::ReplyOnce(ref mut limr,true,_,_,_) => {
        let mut cr = self.origin_read.chain(limr);
        cr.read_end(r)
      }
      ProxyFullKind::ReplyOnce(ref mut limr,ref mut b,_,_,_) => {
        *b = true;
        limr.read_end(r)
      }
    }
  }
}

/// kind of proxying
/// or nothing proxy as previously read (continue reading from origin read)
pub enum ProxyFullKind<SW : ExtWrite, LW : ExtWrite, LR : ExtRead> {
  /// end original read done in proxy init (TODO) and proxy as is then proxy content with symetric enc aka ReplyCached, last is key
  ReplyCached(TunnelCachedWriterExt<SW,LW>,LR),
  /// continue reading from original read and write as is, add cache id if the state if info from
  /// cache or added to cache : aka queryonce, 
  QueryOnce(LW),
  /// proxy content after with sim writer aka ReplyOnce
  ReplyOnce(LR,bool,CompExtW<SW,LW>,LW,bool),
  /// after putting in cache : aka querycache : same as id plus our cach
  QueryCached(Vec<u8>, LW),
}
 
//impl TunnelNoRep for Full {
pub fn clone_error_info<P : Peer, RI : RepInfo, EI : Info + Clone,LW : ExtWrite,TW : TunnelWriterExt>(shads : &mut MultiWExt<TunnelShadowW<P,RI,EI,LW,TW>>) -> Vec<EI> {
      let len : usize = shads.inner_extwrites().len();
      let mut res = Vec::with_capacity(len);
      for i in 0..len {
        match shads.inner_extwrites_mut().get_mut(i) {
          Some(ref mut sh) =>  {
            res.push(sh.err.clone());
          },
          None => panic!("Error not writing multi error info,  will result in dest err read"), // TODO use iter_mut
        }
      }
 
      res.reverse();
      res
}

pub fn clone_shadows_keys<P : Peer, RI : RepInfo, EI : Info,LW : ExtWrite,TW : TunnelWriterExt>(shads : &mut MultiWExt<TunnelShadowW<P,RI,EI,LW,TW>>) -> Vec<Vec<u8>> {
      let len : usize = shads.inner_extwrites().len();
      let mut res = Vec::with_capacity(len);
      for i in 0..len {
        match shads.inner_extwrites_mut().get_mut(i) {
          Some(ref mut sh) =>  {
            res.push(sh.rep.get_reply_key().unwrap().clone()); // TODO no key error
          },
          None => panic!("Error not writing multi sim dest k, will result in erronous read"), // TODO use iter_mut
        }
      }
 
      //res.reverse();
      res
}
impl<TT : GenTunnelTraits> TunnelNoRep for Full<TT> {
  type ReadProv = FullReadProv<TT>;
  type P = TT::P;
  type W = FullW<MultipleReplyInfo<<TT::P as Peer>::Address>, MultipleErrorInfo, TT::P, TT::LW,TT::RW>;
  type TR = FullR<MultipleReplyInfo<<TT::P as Peer>::Address>, MultipleErrorInfo, TT::P, TT::LR>;
  /// actual proxy writer : TODO rem like W : directly return writer default impl when stabilized
  type PW = ProxyFull<Self::TR,TT::SSW,TT::LW,TT::LR>;
  /// Dest reader
  type DR = DestFull<Self::TR,TT::SSR,TT::LR>;
 
  fn new_reader (&mut self) -> Self::TR {

    let s = self.me.new_shadr();
    FullR {
      error_code : None,
      state: TunnelState::TunnelState,
      current_cache_id: None,
      current_reply_info: None,
      current_error_info: None,
      next_proxy_peer : None,
      tunnel_id : None, // TODO useless remove??
      shad : CompExtR(s,self.limiter_proto_r.clone()),
      content_limiter : self.limiter_proto_r.clone(),
      need_content_limiter : false,
      read_cache : false,
    }


  }

  fn init_dest(&mut self, tr : &mut Self::TR) -> Result<()> {
    if tr.next_proxy_peer == None && tr.current_cache_id.is_some() {
      if let TunnelState::ReplyCached = tr.state {
        tr.next_proxy_peer = self.cache.get_symw_tunnel(tr.current_cache_id.as_ref().unwrap()).map(|v|v.1.clone()).ok();
        tr.read_cache = true;
      } else {

      if let TunnelState::QErrorCached = tr.state {
        tr.next_proxy_peer = self.cache.get_errw_tunnel(tr.current_cache_id.as_ref().unwrap()).map(|v|v.1.clone()).ok();
        tr.read_cache = true;
      }
      }
    }
    Ok(())
  }
  // TODO rem redundant cod e with new_wrter_with_route
  #[inline]
  fn new_writer(&mut self, p : &Self::P) -> (Self::W, <Self::P as Peer>::Address) {
    let state = self.get_write_state();
    let (mut shads,next) = self.next_shads(p);
    let ccid = if let TunnelState::QueryCached = state {
      let r = self.new_dest_sym_reader(clone_shadows_keys(&mut shads.0));
      // add reader to cache
      Some(self.put_symr(r).unwrap()) //TODO change trait for error mgmt
    } else {
      self.make_cache_id(state.clone())
    };
    if let MultipleErrorMode::CachedRoute = self.error_mode {
      ccid.as_ref().map_or(Ok(()), |id| {
          let r = clone_error_info(&mut shads.0);
          self.put_errr(id.clone(),r) // TODO change trait
      }).unwrap()
    }
    (FullW {
      current_cache_id : ccid,
      state : state,
      shads: shads,
    }, next)
  }
  #[inline]
  fn new_writer_with_route (&mut self, route : &[&Self::P]) -> Self::W {
  let state = self.get_write_state();
    let mut shads = self.make_shads(route);
    let ccid = if let TunnelState::QueryCached = state {
      let r = self.new_dest_sym_reader(clone_shadows_keys(&mut shads.0));
      // add reader to cache
      Some(self.put_symr(r).unwrap()) //TODO change trait for error mgmt
    } else {
      self.make_cache_id(state.clone())
    };
    if let MultipleErrorMode::CachedRoute = self.error_mode {
      ccid.as_ref().map_or(Ok(()), |id| {
          let r = clone_error_info(&mut shads.0);
          self.put_errr(id.clone(),r) // TODO change trait
      }).unwrap()
    }
    FullW {
      current_cache_id : ccid,
      state : state,
      shads: shads,
    }
  }

  fn new_proxy_writer (&mut self, mut or : Self::TR, from : &<Self::P as Peer>::Address) -> Result<(Self::PW, <Self::P as Peer>::Address)> {
    let (pfk,a) = match or.state {
      TunnelState::ReplyOnce => {
        let key = or.current_reply_info.as_ref().unwrap().get_reply_key().unwrap().clone();
        let ssw = CompExtW(self.sym_prov.new_sym_writer(key),self.limiter_proto_w.clone());
        (ProxyFullKind::ReplyOnce(self.limiter_proto_r.clone(),true,ssw,self.limiter_proto_w.clone(),true), or.next_proxy_peer.clone().unwrap())
      },
      TunnelState::QueryCached => {
        let key = or.current_reply_info.as_ref().unwrap().get_reply_key().unwrap().clone();

        let cache_key = self.new_cache_id();

        if or.current_error_info.as_ref().map(|ei|ei.do_cache()).unwrap_or(false) {
          let (ew,f)= self.new_error_writer(&mut or,from)?;
          self.put_errw(cache_key.clone(),ew,f)?;
        }
        let osk = or.current_cache_id.clone();
//        or.current_cache_id = None; // bad idea (eg error on error from read accessed from proxy)
        let fsk = osk.unwrap();

       
        let ssw = self.new_sym_writer(key,fsk);
        self.put_symw(cache_key.clone(),ssw, from.clone())?;
        (ProxyFullKind::QueryCached(cache_key, self.limiter_proto_w.clone()), or.next_proxy_peer.clone().unwrap())
      },
      TunnelState::ReplyCached => {

        let (tcw,addref) = self.remove_symw(or.current_cache_id.as_ref().unwrap())?;
        (ProxyFullKind::ReplyCached(tcw,or.content_limiter.clone()),addref.clone())
      },
      TunnelState::QueryOnce => {
        (ProxyFullKind::QueryOnce(self.limiter_proto_w.clone()), or.next_proxy_peer.clone().unwrap())
      },
      TunnelState::TunnelState => {
        // TODO remove this state or unpanic by returning error
        panic!("No proxy for this state")
      },

//      TunnelState::QError  => unimplemented!(),
      TunnelState::QErrorCached => unimplemented!(),

    };
    Ok((ProxyFull {
        origin_read : or,
        kind : pfk,
      },a))

  }
  fn new_dest_reader<R : Read> (&mut self, mut or : Self::TR, r : &mut R) -> Result<Self::DR> {
    match or.state {
      TunnelState::TunnelState => {
        // TODO return error of invalid reader state instead of panic
        panic!("invalid reader state")
      },
      TunnelState::QueryOnce => {
        Ok(DestFull {
          origin_read : or,
          kind : DestFullKind::Id,
        })
      },
      TunnelState::QueryCached => {
        // same as query once because no bidirect cache yet (only replycache route use cache and we
        // do not resend) and previous cache id in origin read
        Ok(DestFull {
          origin_read : or,
          kind : DestFullKind::Id,
        })
      },
      TunnelState::ReplyOnce => {

        // TODO this is awkward : move key reading into or method to avoid those lifetime related
        // copies
        let mut current_error_info = or.current_error_info;
        or.current_error_info = None;
        let mut current_reply_info = or.current_reply_info;
        or.current_reply_info = None;
        let mut ks : Vec<Vec<u8>>;
        { 
          let mut inr = CompExtRInner(r, &mut or);
          let len : usize = bin_decode(&mut inr, Infinite).map_err(|e|BincErr(e))?;
          ks = Vec::with_capacity(len);
          for _ in 0..len {
            current_error_info.as_mut().unwrap().read_read_info(&mut inr)?;// should always be init.
            current_reply_info.as_mut().unwrap().read_read_info(&mut inr)?;// should always be init.
            // TODO replace unwrap by return 
            let k : &Vec<u8> = current_reply_info.as_ref().ok_or(IoError::new(IoErrorKind::Other, "unparsed reply info"))?
              .get_reply_key().as_ref().ok_or(IoError::new(IoErrorKind::Other, "no reply key for reply info : wrong reply info"))?;
            ks.push (k.clone());
          }
        }
        let cr = new_dest_cached_reader_ext(ks.into_iter().map(|k|self.sym_prov.new_sym_reader(k)).collect(), self.limiter_proto_r.clone());

        or.current_reply_info = current_reply_info;
        or.current_error_info = current_error_info;
        or.read_end(r)?;
//panic!("reach");
      //let mut buf3 = vec![0;1024];   let a3 = r.read(&mut buf3[..]).unwrap(); panic!("b : {:?}", &buf3[..a3]);
        Ok(DestFull {
          origin_read : or,
          kind : DestFullKind::Multi(cr),
        })
      },
      TunnelState::ReplyCached => {
        // TODO this unwrap can break : remove
        let cr = self.remove_symr(or.current_cache_id.as_ref().unwrap())?;
        let cl = or.content_limiter.clone();
        Ok(DestFull {
          origin_read : or,
          kind : DestFullKind::MultiRc(cr,cl),
        })
      },
      /*TunnelState::QError => {
        unimplemented!()
      },*/
      TunnelState::QErrorCached => {
        unimplemented!()
      },
    }
  }

  fn new_tunnel_read_prov (&self) -> Self::ReadProv {
    FullReadProv {
      me : self.me.clone(),
      limiter_proto_r : self.limiter_proto_r.clone(),
      limiter_proto_w : self.limiter_proto_w.clone(),
      sym_prov : self.sym_prov.clone(),
      reply_once_buf_size : self.reply_once_buf_size,
      _p : PhantomData,
    }
  }
}
impl<TT : GenTunnelTraits> TunnelReadProv<Full<TT>> for FullReadProv<TT> {

  #[inline]
  fn reply_writer_init_init (&mut self) -> Result<Option<<Full<TT> as Tunnel>::RW_INIT>> {
    let lim_payload = self.limiter_proto_r.clone();
    let lim_proxy = self.limiter_proto_w.clone();
    Ok(Some((lim_payload,lim_proxy,self.reply_once_buf_size)))
  }

  /// first option level indicate if it is possible for the provider
  fn new_reply_writer<R : Read> (
    &mut self, 
    tr : &mut <Full<TT> as TunnelNoRep>::DR,
    r : &mut R, 
    ) -> Result<(bool,bool,Option<(<Full<TT> as Tunnel>::RW, <<Full<TT> as TunnelNoRep>::P as Peer>::Address)>)> {
    match *tr.origin_read.current_reply_info.as_ref().unwrap_or_else(
      // TODO return Error instead (when panic removal future refacto)
      || unimplemented!()
    ) {
      MultipleReplyInfo::NoHandling 
      | MultipleReplyInfo::RouteReply(..) => {
        return Ok((false,false,None));
      },
      MultipleReplyInfo::Route => (),
      MultipleReplyInfo::CachedRoute(ref v) => return Ok((true,false,None)),
      _ => unimplemented!(),
    };

     // warn duplicated code TODO function it

     let mut buf = vec![0;self.reply_once_buf_size];
     let mut l;
     // read_end of content
     while {
       l = tr.read_from(r, &mut buf[..])?;
       l != 0
     } {}
     tr.origin_read.switch_toreppayload(r)?;
     let rep;
     let a;
     let need_init;
     {
//      let mut buf3 = vec![0;1024];   let a3 = tr.read_from(r, &mut buf3[..]).unwrap(); panic!("b : {:?}", &buf3[..a3]); // some 72 wrong or from shad simply

        // TODO see if method facto with proxy
        let mut inr = CompExtRInner(r, tr);

        let mut ritmp : MultipleReplyInfo<<TT::P as Peer>::Address> = MultipleReplyInfo::RouteReply(Vec::new());
        ritmp.read_read_info(&mut inr)?;
        // read add
        a = bin_decode(&mut inr, Infinite).map_err(|e|BincErr(e))?;
        rep = if let MultipleReplyInfo::RouteReply(v) = ritmp {
          // return writer
          let ssw = CompExtW(self.sym_prov.new_sym_writer(v),self.limiter_proto_w.clone());
          need_init = true;
          ReplyWriter::Route{shad : ssw}
        } else {
          panic!("missing key for encoding reply"); // TODO transform to error
        };
     }
     Ok((true,need_init,Some((rep,a))))
 
  }
}

impl<TT : GenTunnelTraits> TunnelNoRepReadProv<Full<TT>> for FullReadProv<TT> {

  fn new_tunnel_read_prov (&self) -> Self {
    FullReadProv {
      me : self.me.clone(),
      limiter_proto_r : self.limiter_proto_r.clone(),
      limiter_proto_w : self.limiter_proto_w.clone(),
      sym_prov : self.sym_prov.clone(),
      reply_once_buf_size : self.reply_once_buf_size,
      _p : PhantomData,
    }
  }

  fn new_reader (&mut self) -> <Full<TT> as TunnelNoRep>::TR {
    let s = self.me.new_shadr();
    FullR {
      error_code : None,
      state: TunnelState::TunnelState,
      current_cache_id: None,
      current_reply_info: None,
      current_error_info: None,
      next_proxy_peer : None,
      tunnel_id : None, // TODO useless remove??
      shad : CompExtR(s,self.limiter_proto_r.clone()),
      content_limiter : self.limiter_proto_r.clone(),
      need_content_limiter : false,
      read_cache : false,
    }
  }

  fn can_proxy_writer (&mut self, or : &<Full<TT> as TunnelNoRep>::TR) -> bool { 
   match or.state {
      TunnelState::TunnelState => false,
      TunnelState::QueryOnce => true,
      TunnelState::QueryCached => false,
      TunnelState::ReplyOnce => true,
      TunnelState::ReplyCached => false,
      TunnelState::QErrorCached => false,
    }
  }
  fn new_proxy_writer (&mut self, or : <Full<TT> as TunnelNoRep>::TR) -> Result<Option<(<Full<TT> as TunnelNoRep>::PW, <<Full<TT> as TunnelNoRep>::P as Peer>::Address)>> {
    let (pfk,a) = match or.state {
      TunnelState::ReplyOnce => {
        let key = or.current_reply_info.as_ref().unwrap().get_reply_key().unwrap().clone();
        let ssw = CompExtW(self.sym_prov.new_sym_writer (key),self.limiter_proto_w.clone());
        (ProxyFullKind::ReplyOnce(self.limiter_proto_r.clone(),true,ssw,self.limiter_proto_w.clone(),true), or.next_proxy_peer.clone().unwrap())
      },
      TunnelState::QueryOnce => {
        (ProxyFullKind::QueryOnce(self.limiter_proto_w.clone()), or.next_proxy_peer.clone().unwrap())
      },
      _ => return Ok(None)
    };
    Ok(Some((ProxyFull {
        origin_read : or,
        kind : pfk,
    },a)))
  }
  fn can_dest_reader (&mut self, or : &<Full<TT> as TunnelNoRep>::TR) -> bool {
    match or.state {
      TunnelState::TunnelState => false,
      TunnelState::QueryOnce => true,
      TunnelState::QueryCached => true,
      TunnelState::ReplyOnce => true,
      TunnelState::ReplyCached => false,
      TunnelState::QErrorCached => false,
    }
  }

  fn new_dest_reader<R : Read> (&mut self, mut or : <Full<TT> as TunnelNoRep>::TR, r : &mut R) -> Result<Option<<Full<TT> as TunnelNoRep>::DR>> {
    Ok(match or.state {
      TunnelState::TunnelState => {
        None
      },
      TunnelState::QueryOnce => {
        Some(DestFull {
          origin_read : or,
          kind : DestFullKind::Id,
        })
      },
      TunnelState::QueryCached => {
        // same as query once because no bidirect cache yet (only replycache route use cache and we
        // do not resend) and previous cache id in origin read
        Some(DestFull {
          origin_read : or,
          kind : DestFullKind::Id,
        })
      },
      TunnelState::ReplyOnce => {

        // TODO this is awkward : move key reading into or method to avoid those lifetime related
        // copies TODO duplicate code move in function
        let mut current_error_info = or.current_error_info;
        or.current_error_info = None;
        let mut current_reply_info = or.current_reply_info;
        or.current_reply_info = None;
        let mut ks : Vec<Vec<u8>>;
        { 
          let mut inr = CompExtRInner(r, &mut or);
          let len : usize = bin_decode(&mut inr, Infinite).map_err(|e|BincErr(e))?;
          ks = Vec::with_capacity(len);
          for _ in 0..len {
            current_error_info.as_mut().unwrap().read_read_info(&mut inr)?;// should always be init.
            current_reply_info.as_mut().unwrap().read_read_info(&mut inr)?;// should always be init.
            // TODO replace unwrap by return 
            let k : &Vec<u8> = current_reply_info.as_ref().ok_or(IoError::new(IoErrorKind::Other, "unparsed reply info"))?
              .get_reply_key().as_ref().ok_or(IoError::new(IoErrorKind::Other, "no reply key for reply info : wrong reply info"))?;
            ks.push (k.clone());
          }
        }
        let cr = new_dest_cached_reader_ext(ks.into_iter().map(|k|self.sym_prov.new_sym_reader(k)).collect(), self.limiter_proto_r.clone());

        or.current_reply_info = current_reply_info;
        or.current_error_info = current_error_info;
        or.read_end(r)?;
        Some(DestFull {
          origin_read : or,
          kind : DestFullKind::Multi(cr),
        })
      },
      TunnelState::ReplyCached => {
        None
      },
      TunnelState::QErrorCached => {
        None
      },
    })
  }
}




impl<TT : GenTunnelTraits> Tunnel for Full<TT> {
  // reply info info needed to established conn
  type RI = MultipleReplyInfo<<TT::P as Peer>::Address>;
  /// no info for a reply on a reply (otherwhise establishing sym tunnel seems better)
  type RW = ReplyWriter<TT::LW, TT::SSW>;

//pub struct FullW<RI : RepInfo, EI : Info, P : Peer, LW : ExtWrite,TW : TunnelWriterExt > {
  
  fn new_reply_writer<R : Read> (&mut self, tr : &mut Self::DR, r : &mut R, from : &<Self::P as Peer>::Address) -> Result<Option<(Self::RW, <Self::P as Peer>::Address, bool)>> {

    let (inroute, okey) = match *tr.origin_read.current_reply_info.as_ref().unwrap_or_else(
      // TODO return Error instead (when panic removal future refacto)
      || unimplemented!()
    ) {
      MultipleReplyInfo::NoHandling => {
        return Ok(None);
      },
      MultipleReplyInfo::Route => {
        (true,None)
      },
      MultipleReplyInfo::CachedRoute(ref v) => (false,Some(v.to_vec())),
      _ => unimplemented!(),
    };

     Ok(Some( if inroute {

       let mut buf = vec![0;self.reply_once_buf_size];
       let mut l;
       // read_end of content
       while {
         l = tr.read_from(r, &mut buf[..])?;
         l != 0
       } {}
       tr.origin_read.switch_toreppayload(r)?;
       let rep;
       let a;
       let need_init;
       {
//      let mut buf3 = vec![0;1024];   let a3 = tr.read_from(r, &mut buf3[..]).unwrap(); panic!("b : {:?}", &buf3[..a3]); // some 72 wrong or from shad simply

          // TODO see if method facto with proxy
          let mut inr = CompExtRInner(r, tr);

          let mut ritmp : MultipleReplyInfo<<TT::P as Peer>::Address> = MultipleReplyInfo::RouteReply(Vec::new());
          ritmp.read_read_info(&mut inr)?;
          // read add
          a = bin_decode(&mut inr, Infinite).map_err(|e|BincErr(e))?;
          rep = if let MultipleReplyInfo::RouteReply(v) = ritmp {
            // return writer
            let ssw = CompExtW(self.sym_prov.new_sym_writer(v),self.limiter_proto_w.clone());
            need_init = true;
            ReplyWriter::Route{shad : ssw}
          } else {
            panic!("missing key for encoding reply"); // TODO transform to error
          };
       }
       (rep,a,need_init)


      } else {
        if let Some(k) = okey {
          // return writer TODO consider removing current cache id to avoid clone(not use in reply
          // writer init
          let ssw = TunnelCachedWriterExt::new(self.sym_prov.new_sym_writer(k), tr.origin_read.current_cache_id.clone().unwrap(), self.limiter_proto_w.clone());
          (ReplyWriter::CachedRoute{shad : ssw}, from.clone(),false)

        } else {
          unimplemented!()
         // panic!("missing key for encoding reply"); // TODO transform to error
        }
      }))
 
  }

  type RW_INIT = (<TT as GenTunnelTraits>::LR, <TT as GenTunnelTraits>::LW, usize);

  #[inline]
  fn reply_writer_init_init (&mut self, _ : &mut Self::RW, _ : &mut Self::DR) -> Result<Self::RW_INIT> {
    let lim_payload = self.limiter_proto_r.clone();
    let lim_proxy = self.limiter_proto_w.clone();
    Ok((lim_payload,lim_proxy,self.reply_once_buf_size))
  }

  // TODO consider using plain tr and read_end it (as was done before for new_reply_writer)
  fn reply_writer_init<R : Read, W : Write> (init_init : Self::RW_INIT, repw: &mut Self::RW, tr : &mut Self::DR, r : &mut R, w : &mut W) -> Result<()> {
    let (mut lim_payload, mut lim_proxy,reply_once_buf_size) = init_init;
    if let &mut ReplyWriter::Route{..} = repw {
      let mut inr = CompExtRInner(r, tr);
      let state = bin_decode(&mut inr, Infinite).map_err(|e|BincErr(e))?;
      assert!(if let TunnelState::ReplyOnce = state {true} else {false});
      bin_encode(w, &state, Infinite).map_err(|e|BincErr(e))?;
      lim_payload.read_header(&mut inr)?;
      lim_proxy.write_header(w)?;
      let mut buf = vec![0;reply_once_buf_size];
        // proxy head
        let mut l;
        while {
          l = lim_payload.read_from(&mut inr,&mut buf[..])?;
          lim_proxy.write_all_into(w,&buf[..l])?;
          l != 0
        } {}
          lim_proxy.write_end(w)?;
          lim_payload.read_end(&mut inr)?;
        };
 
    Ok(())
  }
}


impl<TT : GenTunnelTraits> TunnelError for Full<TT> {
  // TODO use a lighter error info type!!!
  type EI = MultipleErrorInfo;
  /// can error on error, cannot reply also, if we need to reply or error on error that is a Reply 
  /// TODO make a specific type for error reply : either cached or replyinfo for full error
  type EW = ErrorWriter;
  fn new_error_writer (&mut self, tr : &mut Self::TR, from : &<Self::P as Peer>::Address) -> Result<(Self::EW, <Self::P as Peer>::Address)> {

    let okey = match *tr.current_error_info.as_ref().unwrap_or_else(
      // TODO return Error instead (when panic removal future refacto)
      || unimplemented!()
    ) {
      MultipleErrorInfo::NoHandling => {
        // TODO return a err (no panic!!)
        unimplemented!()
      },
      MultipleErrorInfo::CachedRoute(ref v) => Some(v),
    };

     Ok( 
        if let Some(k) = okey {
          // return writer TODO consider removing current cache id to avoid clone(not use in reply
          // writer init
          (ErrorWriter::CachedRoute{code : k.clone(), dest_cache_id : tr.current_cache_id.clone().unwrap()}, from.clone())

        } else {
          unimplemented!() // TODO nice error
        })
  }
  fn proxy_error_writer (&mut self, tr : &mut Self::TR, _from : &<Self::P as Peer>::Address) -> Result<(Self::EW, <Self::P as Peer>::Address)> {
    match tr.state {
     
      TunnelState::QErrorCached => {
        let errcode = tr.error_code.clone().unwrap();
        // TODO this unwrap can break : remove
        let (error_cached, dest) = self.get_errw(tr.current_cache_id.as_ref().unwrap())?;

        let ErrorWriter::CachedRoute { code,  dest_cache_id} = error_cached; 
        Ok((ErrorWriter::CachedRoute{code : xor_err_code(errcode, code), dest_cache_id : dest_cache_id.clone()}, dest.clone()))

      },
      _ => {
        // TODO return error of invalid reader state instead of panic
        panic!("invalid reader state")
      },
 
    }
  }


  fn read_error (&mut self, tr : &mut Self::TR) -> Result<usize> {
    match tr.state {
     
      TunnelState::QErrorCached => {
        let errcode = tr.error_code.clone().unwrap();
        // TODO this unwrap can break : remove
        let error_cached = self.get_errr(tr.current_cache_id.as_ref().unwrap())?;

        Ok(identify_cached_errcode(errcode, error_cached)?)

      },
      _ => {
        // TODO return error of invalid reader state instead of panic
        panic!("invalid reader state")
      },
 
    }
  }

}
#[inline]
fn xor_err_code(c1 : usize, c2 : usize) -> usize {
  //println!("xor call : {} with {} = {}", c1, c2, c1 ^ c2);
  c1 ^ c2
}
#[inline]
pub fn identify_cached_errcode (mut err_code : usize, route : &[MultipleErrorInfo]) -> Result<usize> {
  let mut i = 0;
  for ei in &route[..] {
    i += 1;
    if let &MultipleErrorInfo::CachedRoute(c) = ei {
      err_code = xor_err_code(err_code, c);
    }
    if err_code == 0 {
      return Ok(i);
    }
  }
  Err(IoError::new (
    IoErrorKind::Other,
    "Wrong xor error code identifier",
  ))
}

/// Tunnel which allow caching, and thus establishing connections
impl<TT : GenTunnelTraits> TunnelManager for Full<TT> {

  // Shadow Sym (if established con)
  type SSCW = TunnelCachedWriterExt<TT::SSW,TT::LW>;
  // Shadow Sym (if established con)
  type SSCR = MultiRExt<TT::SSR>;

  fn put_symw(&mut self, k : Vec<u8>, w : Self::SSCW, dest : <Self::P as Peer>::Address) -> Result<()> {
    self.cache.put_symw_tunnel(k,(w,dest))
  }

  fn remove_symw(&mut self, k : &Vec<u8>) -> Result<(Self::SSCW,<Self::P as Peer>::Address)> {
    let r = try!(self.cache.remove_symw_tunnel(k));
    Ok(r)
  }
  fn put_symr(&mut self, w : Self::SSCR) -> Result<Vec<u8>> {
    self.cache.put_symr_tunnel(w)
  }

  fn remove_symr(&mut self, k : &Vec<u8>) -> Result<Self::SSCR> {
    let r = self.cache.remove_symr_tunnel(k)?;
    Ok(r)
  }

  fn use_sym_exchange (ri : &Self::RI) -> bool {
    ri.do_cache()
  }

  fn new_sym_writer (&mut self, sk : Vec<u8>, p_cache_id : Vec<u8>) -> Self::SSCW {
    TunnelCachedWriterExt::new(self.sym_prov.new_sym_writer(sk), p_cache_id, self.limiter_proto_w.clone())
  }

  fn new_dest_sym_reader (&mut self, ks : Vec<Vec<u8>>) -> Self::SSCR {
    MultiRExt::new(ks.into_iter().map(|k|self.sym_prov.new_sym_reader(k)).collect())
  }

}

impl<TT : GenTunnelTraits> TunnelManagerError for Full<TT> {
  fn put_errw(&mut self, k : Vec<u8>, w : Self::EW, dest : <Self::P as Peer>::Address) -> Result<()> {
    self.cache.put_errw_tunnel(k,(w,dest))
  }

  fn get_errw(&mut self, k : &Vec<u8>) -> Result<(Self::EW,<Self::P as Peer>::Address)> {
    let r = self.cache.get_errw_tunnel(k)?;
    Ok(r.clone())
  }
  fn put_errr(&mut self, k : Vec<u8>, infos_read : Vec<Self::EI>) -> Result<()> {
    self.cache.put_errr_tunnel(k,infos_read)
  }

  fn get_errr(&mut self, k : &Vec<u8>) -> Result<&[Self::EI]> {
    let r = self.cache.get_errr_tunnel(k)?;
    Ok(&r[..])
  }

}
impl<TT : GenTunnelTraits> CacheIdProducer for Full<TT> {
  fn new_cache_id (&mut self) -> Vec<u8> {
    self.cache.new_cache_id()
  }
}

// This should be split for reuse in last or others (base fn todo)
impl<TT : GenTunnelTraits> Full<TT> {

  /// get state for writing depending on reply
  fn get_write_state (&self) -> TunnelState {
    match self.reply_mode {
      MultipleReplyMode::NoHandling => TunnelState::QueryOnce,
      MultipleReplyMode::KnownDest => TunnelState::QueryOnce,
      MultipleReplyMode::Route => TunnelState::QueryOnce,
      MultipleReplyMode::OtherRoute => TunnelState::QueryOnce,
      MultipleReplyMode::CachedRoute => TunnelState::QueryCached,
      MultipleReplyMode::RouteReply => TunnelState::ReplyOnce,
    }
  }

  // TODO fuse with make_shads (lifetime issue on full : need to open it but first finish
  // make_shads fn
  fn next_shads (&mut self, p : &TT::P) -> (Shadows<TT::P,MultipleReplyInfo<<TT::P as Peer>::Address>,MultipleErrorInfo,TT::LW,TT::RW>, <TT::P as Peer>::Address) {
    let nbpeer;
    let otherroute = if let MultipleReplyMode::OtherRoute = self.reply_mode {true} else{false};
    let revroute : Vec<TT::P>;
    let mut shad : Vec<TunnelShadowW<TT::P,MultipleReplyInfo<<TT::P as Peer>::Address>,MultipleErrorInfo,TT::LW,TT::RW>>;
    let add;
    { // restrict lifetime of peers for other route reply after
      let peers : Vec<&TT::P> = self.route_prov.new_route(p);
      add = peers[1].get_address().clone();
      nbpeer = peers.len();
      shad = Vec::with_capacity(nbpeer - 1);
      let mut errors = self.error_prov.new_error_route(&peers[..]);
      let mut replies = self.reply_prov.new_reply(&peers[..]);
      // TODO rem type
            let mut next_proxy_peer = None;
      let mut geniter = self.rng.gen_iter();

      for i in (1..nbpeer).rev() {
        shad.push(TunnelShadowW {
          shad : peers[i].new_shadw(),
          next_proxy_peer : next_proxy_peer,
          tunnel_id : geniter.next().unwrap(),
          rep : replies.pop().unwrap(),
          err : errors.pop().unwrap(),
          replypayload : None,
        });
        next_proxy_peer = peers.get(i).map(|p|p.get_address().clone());
      }


      assert!(shad.len() > 0);

      if shad.first().map_or(false,|s|

         s.rep.require_additional_payload() && !otherroute) {
           revroute = peers.iter().rev().map(|p|(*p).clone()).collect();
      } else {
        revroute = Vec::new();
      }
    }
    // shad is reversed for MultiW so checking dest is in first
    shad.first_mut().map(|s|
    // set reply payload write for dest
    if s.rep.require_additional_payload() {
       // a bit redundant with require
       let rref = if otherroute {
         self.route_prov.new_reply_route(p)
       } else {
         // messy TODO refactor new_writer to use iterator
         revroute.iter().collect()
       };
       let add = rref[1].get_address().clone();
       s.replypayload = Some((self.limiter_proto_w.clone(),self.tunrep.new_writer_with_route(&rref[..]),add));
    });

    (CompExtW(MultiWExt::new(shad),self.limiter_proto_w.clone()), add)

  }

  fn make_shads (&mut self, peers : &[&TT::P]) -> Shadows<TT::P,MultipleReplyInfo<<TT::P as Peer>::Address>,MultipleErrorInfo,TT::LW,TT::RW> {

    let nbpeer;
    let otherroute = if let MultipleReplyMode::OtherRoute = self.reply_mode {true} else{false};
    let revroute : Vec<TT::P>;
    let mut shad : Vec<TunnelShadowW<TT::P,MultipleReplyInfo<<TT::P as Peer>::Address>,MultipleErrorInfo,TT::LW,TT::RW>>;
    { // restrict lifetime of peers for other route reply after
      nbpeer = peers.len();
      shad = Vec::with_capacity(nbpeer - 1);
      let mut errors = self.error_prov.new_error_route(&peers[..]);
      let mut replies = self.reply_prov.new_reply(&peers[..]);
      //  TODO rem type
      let mut next_proxy_peer = None;
      let mut geniter = self.rng.gen_iter();
//panic!("{:?}",peers);
      for i in (1..nbpeer).rev() {
        shad.push(TunnelShadowW {
          shad : peers[i].new_shadw(),
          next_proxy_peer : next_proxy_peer,
          tunnel_id : geniter.next().unwrap(),
          rep : replies.pop().unwrap(),
          err : errors.pop().unwrap(),
          replypayload : None,
        });
        next_proxy_peer = peers.get(i).map(|p|p.get_address().clone());
      }


      assert!(shad.len() > 0);

      if shad.first().map_or(false,|s|
         s.rep.require_additional_payload() && otherroute) {
           revroute = peers.iter().rev().map(|p|(*p).clone()).collect();
      } else {
        revroute = Vec::new();
      }
    }
    // shad is reversed for MultiW so checking dest is in first
    shad.first_mut().map(|s|
    // set reply payload write for dest
    if s.rep.require_additional_payload() {
       // a bit redundant with require
       let rref = if otherroute {
         self.route_prov.new_reply_route(peers[nbpeer -1])
       } else {
         // messy TODO refactor new_writer to use iterator
         revroute.iter().collect()
       };
       let add = rref[1].get_address().clone();
       s.replypayload = Some((self.limiter_proto_w.clone(),self.tunrep.new_writer_with_route(&rref[..]),add));
    });

    CompExtW(MultiWExt::new(shad),self.limiter_proto_w.clone())

  }


  fn make_cache_id (&mut self, state : TunnelState) -> Option<Vec<u8>> {
    if state.do_cache() {
      Some(self.new_cache_id())
    } else {
      None
    }
  }


}


impl<P : Peer, RI : RepInfo, EI : Info,LW : ExtWrite,TW : TunnelWriterExt> TunnelWriterExt for FullW<RI,EI,P,LW,TW> {

  fn write_dest_info_before<W : Write>(&mut self, w : &mut W) -> Result<()> {
    if let TunnelState::ReplyOnce = self.state {

      // write only emitter key as it is send in tunnelwriter (in additional payload, not from
      // initial tunnelwriter), and should be readable
        match self.shads.0.inner_extwrites_mut().first_mut() {
          Some(ref mut sh) =>  {
            sh.rep.write_read_info(w)?; // TODO shit this write_read trait -> change to get sim key with vec as ref directly

          },
          None => panic!("Error not writing reply sim init k, will result in erronous read"),
        }
    }
    Ok(())
  }


  fn write_dest_info<W : Write>(&mut self, w : &mut W) -> Result<()> {
    if let TunnelState::ReplyOnce = self.state {

      // write all simkeys (inner dest_inof to open multi
      // not all key must be define
      // TODO refactor to avoid cursor : write_read_info and read_read_info are bad, generally the
      // ErrorInfo / ReplyInfo abstraction is bad/useless (this FullW require MultiReply in a lot
      // of place).
      let len : usize = self.shads.0.inner_extwrites().len();

       let mut cw = Cursor::new(Vec::new());
      // TODO do not bin encode usize
      bin_encode(&mut cw, &len, Infinite).map_err(|e|BincErr(e))?;
      // notice that we include key from emitter even if it is already written in dest reply info
      // (duplicated : we should not send it to dest (TODO new enum for dest without key))
      for i in 0..len {
        match self.shads.0.inner_extwrites_mut().get_mut(i) {
          Some(ref mut sh) =>  {
            sh.err.write_read_info(&mut cw)?;
            sh.rep.write_read_info(&mut cw)?;
          },
          None => panic!("Error not writing multi sim dest k, will result in erronous read"),
        }
      }
      self.write_all_into(w, &cw.into_inner()[..])?;
    }
    Ok(())
  }


}
impl<P : Peer, RI : RepInfo, EI : Info,LW : ExtWrite,TW : TunnelWriterExt> ExtWrite for FullW<RI,EI,P,LW,TW> {
  #[inline]
  fn write_header<W : Write>(&mut self, w : &mut W) -> Result<()> {
    // write starte
    bin_encode(w, &self.state, Infinite).map_err(|e|BincErr(e))?;
    // write connect info
    if let Some(cci) = self.current_cache_id.as_ref() {
      bin_encode(w, cci, Infinite).map_err(|e|BincErr(e))?;
    }

    // write connect info 

    // write tunnel header
    self.shads.write_header(w)?;
/*
    // WARNING Big Buff here (only for emitter), with all ri in memory : to keep safe code
    // Only use to write ri and ei info from all shads encoded by all shads
    // TODO overhead could be lower with fix size buffer and redesign of write_dest_info to use
    // buffer instead of write
    // TODO not in read_header (only for dest), could consider reply info containing buffer(bad)
    // TODO might be removable (removal of useless traits)
    let mut buff = Cursor::new(Vec::new());
    self.write_dest_info(&mut buff)?;
    try!(self.write_all_into(w,buff.get_ref()));
    */
    Ok(())
  }


  #[inline]
  fn write_into<W : Write>(&mut self, w : &mut W, cont : &[u8]) -> Result<usize> {
    self.shads.write_into(w,cont)
  }

  #[inline]
  fn write_all_into<W : Write>(&mut self, w : &mut W, cont : &[u8]) -> Result<()> {
    self.shads.write_all_into(w,cont)
  }
  #[inline]
  fn flush_into<W : Write>(&mut self, w : &mut W) -> Result<()> {
    self.shads.flush_into(w)
  }

  #[inline]
  fn write_end<W : Write>(&mut self, w : &mut W) -> Result<()> {
    self.shads.write_end(w)?;

    Ok(())
  }

}

impl<E : ExtRead, P : Peer, RI : RepInfo, EI : Info> FullR<RI,EI,P,E> {
  #[inline]
  fn switch_toreppayload<R : Read>(&mut self, r : &mut R) -> Result<()> {
    if self.need_content_limiter {
      let mut inr  = CompExtRInner(r, &mut self.shad);
      self.content_limiter.read_end(&mut inr)?;
      self.need_content_limiter = false;
    }
    Ok(())
  }


  #[inline]
  pub fn as_read<'a,'b,R : Read>(&'b mut self, r : &'a mut R) -> CompR<'a, 'b, R, FullR<RI,EI,P,E>> {
    CompR::new(r,self)
  }

  /// TODO include in multi read function instead !!
#[inline]
  pub fn read_cacheid<R : Read> (r : &mut R) -> Result<Vec<u8>> {
    Ok(try!(bin_decode(r, Infinite).map_err(|e|BincErr(e))))
  }



}

impl<E : ExtRead, P : Peer, RI : RepInfo, EI : Info> TunnelReaderNoRep for FullR<RI,EI,P,E> {

  fn is_dest(&self) -> Option<bool> {
    match self.state {
      TunnelState::ReplyCached if !self.read_cache => None,
      TunnelState::QErrorCached if !self.read_cache => None,
      TunnelState::TunnelState => None,
      _ => Some(self.next_proxy_peer.is_none()),
    }
  }
  fn is_err(&self) -> Option<bool> {
    match self.state {
      TunnelState::QErrorCached => Some(true),
      TunnelState::TunnelState => None,
      _ => Some(false),
    }
  }
}

impl<E : ExtRead, P : Peer, RI : RepInfo, EI : Info> TunnelReader for FullR<RI,EI,P,E> {
  type RI = RI;
  fn get_current_reply_info(&self) -> Option<&Self::RI> {
    self.current_reply_info.as_ref()
  }
}

impl<E : ExtRead, P : Peer, RI : RepInfo, EI : Info> TunnelReaderError for FullR<RI,EI,P,E> {
  type EI = EI;
  fn get_current_error_info(&self) -> Option<&Self::EI> {
    self.current_error_info.as_ref()
  }
}

impl<E : ExtRead, P : Peer, RI : RepInfo, EI : Info> ExtRead for FullR<RI,EI,P,E> {
  #[inline]
  fn read_header<R : Read>(&mut self, r : &mut R) -> Result<()> {



    // read_state
    self.state = bin_decode(r, Infinite).map_err(|e|BincErr(e))?;

    // read_connect_info
    // reading for cached reader
    if self.state.do_cache() {
      self.current_cache_id = Some(bin_decode(r, Infinite).map_err(|e|BincErr(e))?);
    }

    if let TunnelState::QErrorCached = self.state {
      self.error_code = Some(bin_decode(r, Infinite).map_err(|e|BincErr(e))?);
      return Ok(())
    }

    // read_tunnel_header
    if !self.state.from_cache() {

      self.shad.read_header(r)?;

      let mut inr  = CompExtRInner(r, &mut self.shad);

      self.next_proxy_peer = bin_decode(&mut inr, Infinite).map_err(|e|BincErr(e))?;

      self.tunnel_id = Some(bin_decode(&mut inr, Infinite).map_err(|e|BincErr(e))?);

      self.current_error_info = Some(EI::read_from_header(&mut inr)?);

      let ri = RI::read_from_header(&mut inr)?;

      //try!(self.err.write_in_header(&mut inw));
      //try!(self.rep.write_in_header(&mut inw));

      /*    let mut inw  = CompExtWInner(w, &mut self.shad);
            try!(bin_encode(&self.next_proxy_peer, &mut inw, Infinite).map_err(|e|BincErr(e)));
            */

      if ri.require_additional_payload() {
        self.content_limiter.read_header(&mut inr)?;
        self.need_content_limiter = true;
      };
      self.current_reply_info = Some(ri);

    } else {
      // to allow consume implies that limiter is not in cache
      self.content_limiter.read_header(r)?;
    }

    // read_dest_info // TODO move into  destfull init (dest_read_keys to destfullkind::Multi!!)
    // call in new dest reader i think -> Require refacto where dest reader is init with ref to
    // reader (previous code could also be split in initialiser (state dependant)
    Ok(())
  }

  #[inline]
  fn read_from<R : Read>(&mut self, r : &mut R, buf: &mut [u8]) -> Result<usize> {
 
    if self.state.from_cache() {
      // notice that this implementation is only here to consume content
      self.content_limiter.read_from(r, buf)?;
    }


    if self.need_content_limiter {
      let mut inr  = CompExtRInner(r, &mut self.shad);
      self.content_limiter.read_from(&mut inr,buf)
    } else {
      self.shad.read_from(r,buf)
    }

    /*    if let TunnelMode::NoTunnel = self.mode {
          r.read(buf)
          } else {
          if self.mode.is_het() {
          if let Some(true) = self.is_dest() {
          let mut inw  = CompExtRInner(r, &mut self.shanocont);
          self.shacont.read_from(&mut inw,buf)
          } else {
    // just read header (specific code for content in tunnel proxy function
    self.shadow.read_from(r,buf)
    }
    } else {
    self.shadow.read_from(r,buf)
    }
    }*/
  }

  #[inline]
  fn read_end<R : Read>(&mut self, r : &mut R) -> Result<()> {
    if self.state.from_cache() {
      // notice that this implementation is only here to consume content
      self.content_limiter.read_end(r)?;
    }

    if self.need_content_limiter {
      let mut inr  = CompExtRInner(r, &mut self.shad);
      self.content_limiter.read_end(&mut inr)?;
      self.need_content_limiter = false;
    }

    self.shad.read_end(r)

      /*
      // check if some reply route
      if (self.shadow).1.as_ref().map_or(false,|tpi|if let ErrorHandlingInfo::ErrorRoute = tpi.error_handle {true} else {false}) {

      try!(self.lim.read_end(r));

      }
      Ok(())*/
  }

}

//////-------------

/// override shadow for tunnel writes (additional needed info)
/// First ExtWrite is bytes_wr to use for proxying content (get end of encoded stream).
/// next is possible shadow key for reply,
/// And last is possible shadowroute for reply or error
pub struct TunnelShadowW<P : Peer, RI : Info, EI : Info,LW : ExtWrite,TW : TunnelWriterExt> {
  shad : <P as Peer>::ShadWrite,
  next_proxy_peer : Option<<P as Peer>::Address>,
  tunnel_id : usize, // tunnelId change for every hop that is description tci TODO should only be for cached reply info or err pb same for both : TODO useless ? error code are currently in error info
  rep : RI,
  err : EI,
  replypayload : Option<(LW,TW,<P as Peer>::Address)>,
}

//pub struct TunnelShadowW<E : ExtWrite, P : Peer> (pub <P as Peer>::Shadow, pub TunnelProxyInfo<P>, pub Option<Vec<u8>>, Option<(E,Box<TunnelWriterFull<E,P>>)>);
// TODO switch to TunnelWriter impl and use default extwrite impl
impl<P : Peer, RI : RepInfo, EI : Info,LW : ExtWrite,TW : TunnelWriterExt> ExtWrite for TunnelShadowW<P,RI,EI,LW,TW> {
  #[inline]
  fn write_header<W : Write>(&mut self, w : &mut W) -> Result<()> {
    // write basic tunnelinfo and content
    self.shad.write_header(w)?;
    let mut inw  = CompExtWInner(w, &mut self.shad);
    bin_encode(&mut inw, &self.next_proxy_peer, Infinite).map_err(|e|BincErr(e))?;
    bin_encode(&mut inw, &self.tunnel_id, Infinite).map_err(|e|BincErr(e))?;
    self.err.write_in_header(&mut inw)?;
    self.rep.write_in_header(&mut inw)?;
    if self.rep.require_additional_payload() {
      if let Some((ref mut content_limiter,_,_)) = self.replypayload {
        content_limiter.write_header(&mut inw)?;
      }
    }
    Ok(())
  }
  #[inline]
  fn write_into<W : Write>(&mut self, w : &mut W, cont : &[u8]) -> Result<usize> {
    // this enum is obviously to costy (like general enum : full should specialize for multiple
    // writers (currently trait allow only one : so we need two different full (but still read
    // could be single if on same channel)
    if self.rep.require_additional_payload() {
      let mut inw  = CompExtWInner(w, &mut self.shad);
      if let Some((ref mut content_limiter,_,_)) = self.replypayload {
        return content_limiter.write_into(&mut inw, cont);
      }
    }
    self.shad.write_into(w,cont)
  }   
  #[inline]
  fn flush_into<W : Write>(&mut self, w : &mut W) -> Result<()> {
    if self.rep.require_additional_payload() {
      let mut inw  = CompExtWInner(w, &mut self.shad);
      if let Some((ref mut content_limiter,_,_)) = self.replypayload {
        return content_limiter.flush_into(&mut inw);
      }
    }
    self.shad.flush_into(w)
  }
  #[inline]
  fn write_end<W : Write>(&mut self, w : &mut W) -> Result<()> {
    if self.rep.require_additional_payload() {
      let mut inw  = CompExtWInner(w, &mut self.shad);
      if let Some((ref mut content_limiter,ref mut rr,ref add)) = self.replypayload {
        content_limiter.write_end(&mut inw)?;
        rr.write_dest_info_before(&mut inw)?;
        // write first reply peer address
        bin_encode(&mut inw, add, Infinite).map_err(|e|BincErr(e))?;

        // write header (simkey are include in headers)
        rr.write_header(&mut inw)?;
        // write simkeys to read as dest (Vec<Vec<u8>>)
        // currently same for error
        // put simkeys for reply TODO only if reply expected : move to full reply route ReplyInfo
        // impl -> Reply Info not need to be write into
        rr.write_dest_info(&mut inw)?;

        rr.write_end(&mut inw)?; // warning this call write end 
      }
    }
    self.shad.write_end(w)?;
    Ok(())
/*
    bad design both should be befor write end, but involve another limiter over content -> error or writer with route after changes full behavior : route reply could not be in Info but in TunnelShadowW
      or else do a rep info and err info method toknow if content include after : then use another limiter ~ ok
    Still skip after of reader : test of both include after , if one read to end is called with explicit limiter : write_after method need redesing to have its limiter as parameter

    -- No the plan will be to keep writer in replyinfo (remove latter from error as useless), then do write_after by checking 
    -- reply writer like proxy is base on TR so it can read from it, it also check RI for need of end read but not to read (proxying)
    -- the write info should also be at tunnelShadowW -> remove from reply and error TODO remember new limiter needed for content
    -- first step put reply in tunnel shadow W and link wired write correctly
    -- second step rewrite read for new if (with SR as input)
    -- third correct error and multi

    pb : route provider is related to error ? not really

    // reply or error route TODO having it after had write end is odd (even if at this level we are
    // into another tunnelw shadow), related with read up to reply TODO this might only be ok for
    // error (dif between EI and RI in this case)
    // This is simply that we are added to the end but in various layers.
    // reply or error route TODO having it after had write end is odd (even if at this level we are

    Ok(())*/
  }
}

// --- reader


// ------------ cached sym
pub struct TunnelCachedWriterExt<SW : ExtWrite,E : ExtWrite> {
  shads: CompExtW<SW,E>,
  dest_cache_id: Vec<u8>,
// TODO ??  dest_address: Vec<u8>,
}

impl<SW : ExtWrite,E : ExtWrite> TunnelCachedWriterExt<SW,E> {
  pub fn new (sim : SW, next_cache_id : Vec<u8>, limit : E) -> Self
  {
    TunnelCachedWriterExt {
      shads : CompExtW(sim, limit),
      dest_cache_id : next_cache_id,
    }
  }
}


impl<SW : ExtWrite,E : ExtWrite> ExtWrite for TunnelCachedWriterExt<SW,E> {

  #[inline]
  fn write_header<W : Write>(&mut self, w : &mut W) -> Result<()> {
    bin_encode(w, &TunnelState::ReplyCached, Infinite).map_err(|e|BincErr(e))?;
    bin_encode(w, &self.dest_cache_id, Infinite).map_err(|e|BincErr(e))?;
    self.shads.write_header(w)
  }
  #[inline]
  fn write_into<W : Write>(&mut self, w : &mut W, cont: &[u8]) -> Result<usize> {
    self.shads.write_into(w,cont)
  }
  #[inline]
  fn write_all_into<W : Write>(&mut self, w : &mut W, buf : &[u8]) -> Result<()> {
    self.shads.write_all_into(w, buf)
  }
  #[inline]
  fn flush_into<W : Write>(&mut self, w : &mut W) -> Result<()> {
    self.shads.flush_into(w)
  }
  #[inline]
  fn write_end<W : Write>(&mut self, w : &mut W) -> Result<()> {
    self.shads.write_end(w)
  }

}

impl<OR : ExtRead,SW : ExtWrite, E : ExtWrite,LR : ExtRead> TunnelWriterExt for ProxyFull<OR,SW,E,LR> {
  #[inline]
  fn write_dest_info_before<W : Write>(&mut self, _ : &mut W) -> Result<()> {
    Ok(())
  }

  #[inline]
  fn write_dest_info<W : Write>(&mut self, _ : &mut W) -> Result<()> {
    Ok(())
  }
}
impl<OR : ExtRead,SW : ExtWrite, E : ExtWrite,LR : ExtRead> ExtWrite for ProxyFull<OR,SW,E,LR> {
  #[inline]
  fn write_header<W : Write>(&mut self, w : &mut W) -> Result<()> {
   // let mut wc = Cursor::new(Vec::new());
   // { let mut wa = &mut wc;
    // write state
    match self.kind {
      ProxyFullKind::ReplyCached(_,_) => 
     (), //  bin_encode(&TunnelState::ReplyCached, wa, Infinite).map_err(|e|BincErr(e))?,
      ProxyFullKind::QueryCached(_,_) =>
        bin_encode(w, &TunnelState::QueryCached, Infinite).map_err(|e|BincErr(e))?,
      ProxyFullKind::QueryOnce(_) =>
        bin_encode(w, &TunnelState::QueryOnce, Infinite).map_err(|e|BincErr(e))?,
      ProxyFullKind::ReplyOnce(_,_,_,_,_) =>
        bin_encode(w, &TunnelState::ReplyOnce, Infinite).map_err(|e|BincErr(e))?,
    }
   
   

    // write connnect_info (cf do_cache method of full)
    match self.kind {
      ProxyFullKind::ReplyCached(_,_) => {
    //    let k = &c.try_borrow().map_err(|e|BorrErr(e))?.dest_cache_id;
    //    bin_encode(k, wa, Infinite).map_err(|e|BincErr(e))?;
      },
      ProxyFullKind::QueryOnce(_) => (),
      ProxyFullKind::ReplyOnce(_,_,_,_,_) => (),
      ProxyFullKind::QueryCached(ref k,_) =>
        bin_encode(w, k, Infinite).map_err(|e|BincErr(e))?,
    }
   
    // write tunnel header
    match self.kind {
      ProxyFullKind::ReplyCached(ref mut inner,_) => inner.write_header(w)?,
      ProxyFullKind::ReplyOnce(_,_,_,ref mut lim,_) | ProxyFullKind::QueryOnce(ref mut lim) | ProxyFullKind::QueryCached(_,ref mut lim) => lim.write_header(w)?,
    }
    //}if let ProxyFullKind::ReplyCached(..) = self.kind {panic!("{:?}",wc.into_inner());}
    Ok(())
  }


  fn write_into<W : Write>(&mut self, w : &mut W, buf : &[u8]) -> Result<usize> {
    match self.kind {
      ProxyFullKind::ReplyCached(ref mut inner,_) => inner.write_into(w,buf),
      ProxyFullKind::QueryOnce(ref mut lim) | ProxyFullKind::QueryCached(_,ref mut lim) => lim.write_into(w,buf),

      // in header proxying and in header reading
      ProxyFullKind::ReplyOnce(_,true,_,ref mut lim,true) => {
        lim.write_into(w,buf)
      },
      // read in payload
      ProxyFullKind::ReplyOnce(_,false,ref mut payw,ref mut lim,ref mut b) => {
        // require to switch
        if *b {
          lim.write_end(w)?;
          payw.write_header(w)?;
          *b = false;
        }
        payw.write_into(w,buf)
      },
      // TODO use a single state to avoid this
      ProxyFullKind::ReplyOnce(_,true,_,_,false) => panic!("Inconsistent replyonce proxy state read in header and write in content"),

    }
  }

  fn write_all_into<W : Write>(&mut self, w : &mut W, buf : &[u8]) -> Result<()> {
    match self.kind {
      ProxyFullKind::ReplyCached(ref mut inner,_) => inner.write_all_into(w,buf),
      ProxyFullKind::QueryOnce(ref mut lim) | ProxyFullKind::QueryCached(_, ref mut lim) => lim.write_all_into(w,buf),
      // a bit suboptimal as default impl
      ProxyFullKind::ReplyOnce(..) => {
        let mut cdw = CompExtW(DefaultID(), self);
        cdw.write_all_into(w,buf)
      },
    }
  }

  /// ExtWrite flush into
  fn flush_into<W : Write>(&mut self, w : &mut W) -> Result<()> {
    match self.kind {
      ProxyFullKind::ReplyCached(ref mut inner,_) => inner.flush_into(w),
      ProxyFullKind::QueryOnce(ref mut lim) | ProxyFullKind::QueryCached(_, ref mut lim) => lim.flush_into(w),
      ProxyFullKind::ReplyOnce(_,_,_,ref mut lim,true) => {
        lim.flush_into(w)
      },
      ProxyFullKind::ReplyOnce(_,_,ref mut payw,_,false) => {
        payw.flush_into(w)
      },
    }
  }
  /// ExtWrite write end
  fn write_end<W : Write>(&mut self, w : &mut W) -> Result<()> {
    match self.kind {
      ProxyFullKind::ReplyCached(ref mut inner,_) => inner.write_end(w),
      ProxyFullKind::QueryOnce(ref mut lim) | ProxyFullKind::QueryCached(_, ref mut lim) => lim.write_end(w),
      ProxyFullKind::ReplyOnce(_,_,ref mut payw,ref mut lim,true) => {
        lim.write_end(w)?;
        payw.write_header(w)?;
        payw.write_end(w)
      },
      ProxyFullKind::ReplyOnce(_,_,ref mut payw,_,ref mut b) => {
        *b = true;
        payw.write_end(w)
      },
    }
  }
}

/**
 * reader for a dest of tunnel cached
 */
pub type TunnelCachedReaderExt<SR,E> = CompExtR<MultiRExt<SR>,E>;

fn new_dest_cached_reader_ext<SR : ExtRead, E : ExtRead> (sim : Vec<SR>, limit : E) -> TunnelCachedReaderExt<SR,E> {
  CompExtR(MultiRExt::new(sim), limit)
}


pub struct DestFull<OR : ExtRead,SR : ExtRead, E : ExtRead> {
  pub origin_read : OR,
  pub kind : DestFullKind<SR,E>
}
impl<OR : ExtRead,SR : ExtRead, E : ExtRead> TunnelReaderExt for DestFull<OR,SR,E> {
  type TR = OR; 
  fn get_reader(self) -> Self::TR {
    self.origin_read
  }
}

impl<OR : ExtRead,SR : ExtRead, E : ExtRead> ExtRead for DestFull<OR,SR,E> {
  fn read_header<R : Read>(&mut self, r : &mut R) -> Result<()> {
    match self.kind {
      DestFullKind::Multi(ref mut rs) => rs.read_header(r),
      DestFullKind::MultiRc(ref mut rs, ref mut lim) => {
        let mut inr = CompExtRInner(r, lim);
        rs.read_header(&mut inr)
      },
      DestFullKind::Id => Ok(()),
    }
  }

  fn read_from<R : Read>(&mut self, r : &mut R, buf : &mut[u8]) -> Result<usize> {
    match self.kind {
      DestFullKind::Multi(ref mut rs) => rs.read_from(r,buf),
      DestFullKind::MultiRc(ref mut rs, ref mut lim) => {
        let mut inr = CompExtRInner(r, lim);
        rs.read_from(&mut inr,buf)
      },
      DestFullKind::Id => self.origin_read.read_from(r,buf),
    }
  }

  fn read_exact_from<R : Read>(&mut self, r : &mut R, buf: &mut[u8]) -> Result<()> {
    match self.kind {
      DestFullKind::Multi(ref mut rs) => rs.read_exact_from(r, buf),
      DestFullKind::MultiRc(ref mut rs, ref mut lim) => {
        let mut inr = CompExtRInner(r, lim);
        rs.read_exact_from(&mut inr,buf)
      },
      DestFullKind::Id => self.origin_read.read_exact_from(r,buf),
    }
  }

  fn read_end<R : Read>(&mut self, r : &mut R) -> Result<()> {
    match self.kind {
      DestFullKind::Multi(ref mut rs) => rs.read_end(r),
      DestFullKind::MultiRc(ref mut rs, ref mut lim) => {
        { let mut inr = CompExtRInner(r, lim);
        rs.read_end(&mut inr)?;
        }
        lim.read_end(r)
      },
      DestFullKind::Id => self.origin_read.read_end(r),
    }
  }
}

/// kind of destreader
pub enum DestFullKind<SR : ExtRead, LR : ExtRead> {
  /// Multi : for reply
  Multi(TunnelCachedReaderExt<SR,LR>),
  MultiRc(MultiRExt<SR>,LR),
  /// Nothing directly ok
  Id,
}


// should be split like Full, furthermore similar to proxy impl (some code may be shared?)
pub enum ReplyWriter<LW : ExtWrite, SW : ExtWrite> {
  Route {
    // reply shadower
    shad : CompExtW<SW,LW>,
  },
  CachedRoute {
    // reply shadower
    shad : TunnelCachedWriterExt<SW,LW>,
  },
}
#[derive(Clone)]
pub enum ErrorWriter {
  CachedRoute {
    code : usize,
    dest_cache_id : Vec<u8>, // TODO could use &'[u8], depends on use case
  },
}


impl<LW : ExtWrite, SW : ExtWrite> TunnelWriterExt for ReplyWriter<LW,SW> {
  fn write_dest_info_before<W : Write>(&mut self, _ : &mut W) -> Result<()> {
    match *self {
      ReplyWriter::Route { shad : _ } => Ok(()),
      ReplyWriter::CachedRoute { shad : _ } => Ok(()),
    }
  }

  fn write_dest_info<W : Write>(&mut self, _ : &mut W) -> Result<()> {
    match *self {
      ReplyWriter::Route { shad : _ } => Ok(()),
      ReplyWriter::CachedRoute { shad : _ } => Ok(()),
    }
  }
}


impl<LW : ExtWrite, SW : ExtWrite> ExtWrite for ReplyWriter<LW,SW> {

  fn write_header<W : Write>(&mut self, w : &mut W) -> Result<()> {
    match *self {
      ReplyWriter::Route { ref mut shad } => shad.write_header(w),
      ReplyWriter::CachedRoute { ref mut shad } => shad.write_header(w), // TODO write state ri, ei and others (key of dest (add TODO read in prev)
    }
  }
 
  fn write_into<W : Write>(&mut self, w : &mut W, cont : &[u8]) -> Result<usize> {
    match *self {
      ReplyWriter::Route { ref mut shad } => shad.write_into(w,cont),
      ReplyWriter::CachedRoute { ref mut shad } => shad.write_into(w,cont),
    }
  }
  fn write_all_into<W : Write>(&mut self, w : &mut W, cont : &[u8]) -> Result<()> {
    match *self {
      ReplyWriter::Route { ref mut shad } => shad.write_all_into(w,cont),
      ReplyWriter::CachedRoute { ref mut shad } => shad.write_all_into(w,cont),
    }
  }
 
 
  fn flush_into<W : Write>(&mut self, w : &mut W) -> Result<()> {
    match *self {
      ReplyWriter::Route { ref mut shad } => shad.flush_into(w),
      ReplyWriter::CachedRoute { ref mut shad } => shad.flush_into(w),
    }
  }
 
  fn write_end<W : Write>(&mut self, w : &mut W) -> Result<()> {
    match *self {
      ReplyWriter::Route { ref mut shad } => shad.write_end(w),
      ReplyWriter::CachedRoute { ref mut shad } => shad.write_end(w),
    }
  }
 
}

impl TunnelErrorWriter for ErrorWriter {
  fn write_error<W : Write>(&mut self, w : &mut W) -> Result<()> {
    match *self {
      ErrorWriter::CachedRoute { code : ref c, dest_cache_id : ref dcid } => {
//   TunnelState tci1 (1 errorcode3)
        bin_encode(w, &TunnelState::QErrorCached, Infinite).map_err(|e|BincErr(e))?;
        bin_encode(w, dcid, Infinite).map_err(|e|BincErr(e))?;
        bin_encode(w, c, Infinite).map_err(|e|BincErr(e))?;
      }
    }
    Ok(())
  }
}

