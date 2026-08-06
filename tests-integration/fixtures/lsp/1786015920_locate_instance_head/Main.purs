module Main where

import Lib as Lib

class Functor f
--    $ @ %

data Identity a = Identity a

instance Functor Identity
--       $@%&/?

foreign import functor :: forall f. Functor f => f

data Derived = Derived

derive instance Functor Derived
--              $@%

newtype Wrapper = Wrapper Identity

derive newtype instance Functor Wrapper
--                      $@%

data ChainFirst = ChainFirst

data ChainSecond = ChainSecond

instance Functor ChainFirst
else instance Lib.ImportedFunctor ChainSecond
--                $@%/
