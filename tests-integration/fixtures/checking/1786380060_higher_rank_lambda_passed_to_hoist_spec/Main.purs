module Main where

infixl 1 apply as #

data Unit = Unit

data First :: Type -> Type
data First a = First

data Second :: Type -> Type
data Second a = Second

data Third :: Type -> Type
data Third a = Third

data SpecT :: (Type -> Type) -> (Type -> Type) -> Type
data SpecT effect monad = SpecT

type NaturalTransformation f g = forall a. f a -> g a

identity :: forall a. a -> a
identity x = x

apply :: forall a b. a -> (a -> b) -> b
apply x function = function x

describe :: forall effect monad. String -> SpecT effect monad -> SpecT effect monad
describe _ specification = specification

it :: forall effect monad. String -> effect Unit -> SpecT effect monad
it _ _ = SpecT

hoistSpec
  :: forall first second monad monad'
   . NaturalTransformation monad monad'
  -> (Unit -> NaturalTransformation first second)
  -> SpecT first monad
  -> SpecT second monad'
hoistSpec _ _ _ = SpecT

catchFirst :: NaturalTransformation First Second
catchFirst _ = Second

catchSecond :: NaturalTransformation Second Third
catchSecond _ = Third

runExample :: First Unit
runExample = First

spec :: SpecT Third First
spec =
  describe "outer" do
    it "inner" runExample
    # hoistSpec identity
        ( \_ value -> value
            # catchFirst
            # catchSecond
        )
