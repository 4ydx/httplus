## Http request parser

This is rudimentary.

The high level idea is:

1. Create a new Request object.
2. Receive streaming HTTP request data from some external source.
3. As the data comes in, call request.update, passing in streamed data.

Eventually all of the data (D) will be fed into request.update(D) then:

1. request.body_complete() will return true.
2. request.body() will return the body captured based on the content-length header.

## Notes

https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers
https://datatracker.ietf.org/doc/html/rfc7230
https://datatracker.ietf.org/doc/html/rfc7540
https://datatracker.ietf.org/doc/rfc9114/
