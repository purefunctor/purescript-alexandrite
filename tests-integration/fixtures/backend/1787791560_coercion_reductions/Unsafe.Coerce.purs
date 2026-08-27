module Unsafe.Coerce where

foreign import unsafeCoerce :: forall source target. source -> target
