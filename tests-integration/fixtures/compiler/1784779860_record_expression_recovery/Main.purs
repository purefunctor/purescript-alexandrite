module Main where

missingField :: { valid :: Int }
missingField = { valid: 1, missing: }

unresolvedPun = { missing }
