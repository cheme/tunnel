//! Module containing default implementation.
//! For instance default trait implementation to import in order to have a full tunnel
//! implementation even if the tunnel only implement send only or do not manage error
use super::{
  TunnelWriter,
  Info,
};

use super::common::{
  TunnelState,
};

