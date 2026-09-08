module Main where

type Optional :: forall k. Row k
type Optional = ()

type All =
  ( value :: Int
  | Optional
  )

defaultOptions :: { | Optional }
defaultOptions = {}

class ConvertOptions defaults provided all

instance
  ConvertOptions
    { | Optional }
    { | provided }
    { | All }
