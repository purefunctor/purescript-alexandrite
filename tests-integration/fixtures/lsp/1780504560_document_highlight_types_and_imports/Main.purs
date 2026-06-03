module Main where

import Lib (value, Box, class ImportedClass, type (:*:), (<++>))
--          &      &    &                    &           &

foreign import data Local :: Type
--                  &

infixr 6 type Local as :+:
--                    &

data LocalData = LocalData
--   &

localData = LocalData
--          &

newtype LocalNewtype = LocalNewtype Local
--      &

class LocalClass a where
--    &
  localMember :: a -> Local
--  &

foreign import localForeign :: Local
--             &

instance localClassLocal :: LocalClass Local where
--       &
  localMember value = value

plus left right = left

infixr 5 plus as <+>
--       &        &

type LocalAlias = Local
--   &          &

importedValue = value
--              &

importedOperator = value <++> value
--                       &

localOperator = localForeign <+> localForeign
--                           &

foreign import importedClassUse :: forall a. ImportedClass a => a -> a
--                                           &

foreign import localClassUse :: forall a. LocalClass a => a -> a
--                                        &

type ImportedBox = Box
--   &             &

type ImportedOperator = Box :*: Box
--   &                    &   &

type LocalOperator = Local :+: Local
--                         &
