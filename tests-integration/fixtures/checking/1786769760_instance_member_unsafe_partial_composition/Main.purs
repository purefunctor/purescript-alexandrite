module Main where

import Data.Semigroupoid ((<<<))
import Partial.Unsafe (unsafePartial)

data Content = Sentence String | Paragraph (Array String)

class ContentOverload a where
  fromContent :: Content -> a
  fromContentPartialLambda :: Content -> a
  fromContentLambdaPartial :: Content -> a

instance ContentOverload String where
  fromContent = unsafePartial <<< case _ of
    Sentence content -> content
  fromContentPartialLambda = unsafePartial \content -> case content of
    Sentence value -> value
  fromContentLambdaPartial = \content -> unsafePartial case content of
    Sentence value -> value

instance ContentOverload (Array String) where
  fromContent = unsafePartial <<< case _ of
    Paragraph content -> content
  fromContentPartialLambda = unsafePartial \content -> case content of
    Paragraph value -> value
  fromContentLambdaPartial = \content -> unsafePartial case content of
    Paragraph value -> value
