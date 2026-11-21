# Bounds

ids stay below 2^31 and coordinates within 10^6 bond lengths and finite (`MAX_ID`, `MAX_COORD`): a document or an operation outside them is refused (`error.draw2.idTaken`, `error.draw2.coordinate`), so id arithmetic cannot overflow and no number that JSON cannot carry enters a document. Hostile input (deep nesting, huge arrays, long strings, extreme numbers) is refused without a panic, by test.
