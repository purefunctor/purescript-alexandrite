module Main where

import Prim.Int (class Add)
import Prim.Row as Row
import Type.Proxy (Proxy(..))

needsAdd :: forall x. Add 1 x 3 => Proxy x
needsAdd = Proxy

class IsInt value
instance IsInt Int

needsInt :: forall value. IsInt value => Proxy value
needsInt = Proxy

nonHeadGivenImprovement
  :: forall n
   . Row.Union (a :: Proxy n) () (a :: Proxy 2)
  => Proxy n
nonHeadGivenImprovement = needsAdd

functionGivenImprovement
  :: forall value
   . Row.Union (a :: value -> String) () (a :: Int -> String)
  => Proxy value
functionGivenImprovement = needsInt

forceSolve =
  { nonHeadGivenImprovement
  , functionGivenImprovement
  }
