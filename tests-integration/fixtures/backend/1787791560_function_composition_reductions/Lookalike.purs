module Lookalike where

compose :: forall value. (value -> value) -> (value -> value) -> value -> value
compose outer inner value = outer (inner value)

composeFlipped :: forall value. (value -> value) -> (value -> value) -> value -> value
composeFlipped inner outer value = outer (inner value)
