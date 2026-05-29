module Main where

-- !

foreign import data ForeignType :: Type

foreign import foreignValue :: Int

type Synonym = Int

newtype Id a = Id a

class Wrap a where
  wrap :: a -> a

instance wrapInt :: Wrap Int where
  wrap x = x

derive newtype instance wrapId :: Wrap a => Wrap (Id a)

plus :: Int -> Int -> Int
plus x y = x

infixl 5 plus as ++

type Add a b = Int

infixl 5 type Add as :+:
