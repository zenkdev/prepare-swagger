import test from 'ava'

import { ping } from '../index'

test('sync function from native code', (t) => {
  const actual = ping()

  t.is(actual, 'pong')
})
