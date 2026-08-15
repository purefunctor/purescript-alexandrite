module Main where

class Wrap a

data Box = Box

derive instance derivedWrap :: Wrap Box
--              $@%&/?
