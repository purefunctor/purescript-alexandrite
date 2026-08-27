module Lookalike where

apply :: forall argument result. (argument -> result) -> argument -> result
apply function argument = function argument
