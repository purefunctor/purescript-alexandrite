module Safe.Coerce where

import Prim.Coerce (class Coercible)
import Unsafe.Coerce (unsafeCoerce)

coerce :: forall source target. Coercible source target => source -> target
coerce = unsafeCoerce
