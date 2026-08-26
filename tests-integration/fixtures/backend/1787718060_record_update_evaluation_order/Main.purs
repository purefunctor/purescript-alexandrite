module Main where

type Nested =
  { value :: Int
  , untouched :: Int
  }

type Model =
  { first :: Int
  , nested :: Nested
  , last :: Int
  , untouched :: Int
  }

foreign import failAt :: String -> Boolean -> Int -> Int

foreign import makeModel :: String -> Model

foreign import observe :: String -> Int -> Int

updateLocal :: Model -> Model
updateLocal source =
  source
    { first = observe "local-first" 10
    , nested { value = observe "local-nested" 20 }
    , last = observe "local-last" 30
    }

updateCall :: Boolean -> Model
updateCall shouldThrow =
  (makeModel "call")
    { first = observe "call-first" 10
    , nested { value = failAt "call-nested" shouldThrow 20 }
    , last = observe "call-last" 30
    }

updateControl :: Boolean -> Model -> Model
updateControl condition source =
  source
    { first = observe "control-first" 10
    , last =
        if condition then observe "control-then" 30
        else observe "control-else" 31
    }

updateOpen
  :: forall row
   . { first :: Int | row }
  -> { first :: Int | row }
updateOpen source = source { first = observe "open-first" 40 }
