module Main where

import Prim.Int (class Add)

data Proxy :: forall k. k -> Type
data Proxy a = Proxy

class Base (value :: Type) where
  base :: Int

instance baseInt :: Base Int where
  base = 42

class Chain (value :: Type) (number :: Int) where
  chain :: Int

instance chainZero :: Base value => Chain value 0 where
  chain = base @value
else instance chainNext
  :: (Add previous 1 current, Chain value previous)
  => Chain value current where
  chain = 0

chainValue
  :: forall value number
   . Chain value number
  => Proxy value
  -> Proxy number
  -> Int
chainValue _ _ = chain @value @number

useTwice :: forall value. Base value => Proxy value -> Array Int
useTwice proxy =
  [ chainValue proxy (Proxy :: Proxy 70)
  , chainValue proxy (Proxy :: Proxy 70)
  ]

result :: Array Int
result = useTwice (Proxy :: Proxy Int)
