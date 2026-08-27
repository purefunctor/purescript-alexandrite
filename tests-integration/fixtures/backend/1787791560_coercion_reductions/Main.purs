module Main where

import Lookalike as Lookalike
import Safe.Coerce as SafeCoerce
import Unsafe.Coerce as UnsafeCoerce

unsafelyCoerced :: Int
unsafelyCoerced = UnsafeCoerce.unsafeCoerce 42

safelyCoerced :: Int
safelyCoerced = SafeCoerce.coerce 42

lookalikeUnsafeCoerce :: Int
lookalikeUnsafeCoerce = Lookalike.unsafeCoerce 42

lookalikeSafeCoerce :: Int
lookalikeSafeCoerce = Lookalike.coerce 42
