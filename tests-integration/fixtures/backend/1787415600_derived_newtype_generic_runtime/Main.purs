module Main where

import Data.Generic.Rep (class Generic, from, to)
import Data.Newtype (class Newtype, unwrap)

newtype Identifier = Identifier Int

derive instance Newtype Identifier _

wrapped :: Identifier
wrapped = Identifier 42

unwrapped :: Int
unwrapped = unwrap wrapped

data Choice = Empty | Single Int | Pair Int Int

derive instance Generic Choice _

roundTrip :: Choice -> Choice
roundTrip value = to (from value)

emptyRoundTrip :: Choice
emptyRoundTrip = roundTrip Empty

singleRoundTrip :: Choice
singleRoundTrip = roundTrip (Single 6)

pairRoundTrip :: Choice
pairRoundTrip = roundTrip (Pair 7 8)

data Void

derive instance Generic Void _
