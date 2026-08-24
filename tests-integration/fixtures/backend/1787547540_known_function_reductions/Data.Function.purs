module Data.Function where

apply :: forall argument result. (argument -> result) -> argument -> result
apply function argument = function argument

applyFlipped :: forall argument result. argument -> (argument -> result) -> result
applyFlipped argument function = function argument
