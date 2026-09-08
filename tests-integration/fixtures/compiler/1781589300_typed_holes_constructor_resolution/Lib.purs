module Lib where

data Remote = Shared | RemoteOnly

remoteValue :: Remote
remoteValue = RemoteOnly

shadowedValue :: Remote
shadowedValue = RemoteOnly
