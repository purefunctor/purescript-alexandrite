module Lookalike where

apply :: forall argument result. (argument -> result) -> argument -> result
apply function argument = function argument

class Identity (identity :: Type -> Type -> Type) where
  identity :: forall value. identity value value

instance categoryFn :: Identity Function where
  identity value = value

unsafeCoerce :: forall value. value -> value
unsafeCoerce value = value
