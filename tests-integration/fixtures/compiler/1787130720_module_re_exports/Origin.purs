module Origin
  ( Option(Just)
  , Wrapped(..)
  , await
  , class Measure
  , foreignValue
  , type (:*:)
  , visible
  , (<>)
  ) where

import Data.Eq (class Eq)

foreign import foreignValue :: Int

visible :: Int -> Int
visible value = value

hidden :: Int
hidden = 13

await :: Int
await = 17

data Option = Just Int | Nothing

derive instance Eq Option

newtype Wrapped = Wrapped Int

append :: Int -> Int -> Int
append left _ = left

infixr 5 append as <>

type Product left right = left

infixr 6 type Product as :*:

class Measure a where
  measure :: a -> Int

instance measureInt :: Measure Int where
  measure value = value
