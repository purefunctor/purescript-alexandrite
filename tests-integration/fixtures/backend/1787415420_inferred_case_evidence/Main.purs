module Main where

class Empty collection where
  empty :: forall value. collection value

chooseEmpty :: forall collection value. Empty collection => Boolean -> collection value
chooseEmpty = case _ of
  true -> empty
  false -> empty
