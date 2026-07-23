module Main where

class First :: Type -> Constraint
class First a

class Second :: Type -> Constraint
class Second a

foreign import interleaved :: forall a. First a => (forall b. Second b => a -> b)

inferredPun = { interleaved }

inferredExplicit = { interleaved: interleaved }

type Alias = Int

aliased :: Alias
aliased = 1

inferredAliasPun = { aliased }

inferredAliasExplicit = { aliased: aliased }

class Capability :: (Type -> Type) -> Constraint
class Capability m

foreign import fetch :: forall m. Capability m => Int -> m Int

type Setup =
  { fetch :: forall m. Capability m => Int -> m Int
  }

expectedPun :: Setup
expectedPun = { fetch }

expectedExplicit :: Setup
expectedExplicit = { fetch: fetch }
