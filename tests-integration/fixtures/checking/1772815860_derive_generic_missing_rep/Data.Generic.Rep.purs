module Data.Generic.Rep where

class Generic a rep | a -> rep where
  to :: rep -> a
  from :: a -> rep
