module Main where

import Control.Applicative (pure)
import Control.Bind (bind)
import Control.Monad.ST.Internal (ST, run)
import Data.Unit (Unit(..))

namedContinuation :: forall region. Unit -> ST region Int
namedContinuation _ = pure 42

namedBind :: forall region. Unit -> ST region Int
namedBind _ = bind (pure Unit) namedContinuation

runNamedBind :: Unit -> Int
runNamedBind _ = run (namedBind Unit)
