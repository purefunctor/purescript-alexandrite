module Lookalike where

unsafeCoerce :: forall value. value -> value
unsafeCoerce value = value

coerce :: forall value. value -> value
coerce value = value
