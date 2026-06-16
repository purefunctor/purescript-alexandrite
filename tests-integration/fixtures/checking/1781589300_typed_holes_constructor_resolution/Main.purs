module Main where

import Lib

data Local = Shared | LocalOnly

localValue :: Local
localValue = LocalOnly

shadowedValue :: Local
shadowedValue = LocalOnly

local :: Local
local = ?local

remote :: Remote
remote = ?remote

binderShadow :: Local -> Remote
binderShadow remoteValue = ?binder
