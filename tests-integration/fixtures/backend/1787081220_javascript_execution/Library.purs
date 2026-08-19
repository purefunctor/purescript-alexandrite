module Library where

data Wrapped = Wrapped Int

wrapped :: Wrapped
wrapped = Wrapped 21

forward :: Int
forward = later

later :: Int
later = 13
