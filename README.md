## Http request parser

This is rudimentary.

The high level idea is:

1. Create a new Request object.
2. Receive streaming HTTP request data from some external source.
3. As the data comes in, call request.update, passing in streamed data.

Eventually all of the data D will be fed into request.update(D) and request.body_complete()
will return true for all request types. That is, any request that specifies a
"Content-Length" header will have that request.body() data available. Otherwise, the
request.body() will be empty.
