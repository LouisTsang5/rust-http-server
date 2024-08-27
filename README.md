# HTTP SERVER

A http file server. The program reads the file content based on the path given in a HTTP GET request and returns its content. Useful for http service / API mocking.

## Compilation

To build the executable. Run the following command

```rust
cargo build --release
```

## HTTP Response

Responses are created by reading files within the ```res``` folder using relative path. There should be a folder named ```res``` at the ```PWD```.

For example, a request for path ```/inner/res``` will be mapped to the file ```./res/inner/res``` (no file extension). If the file does not exists, ```404 NOT FOUND``` is returned.

If the mapped file_path is a directory, a read attempt is made to the file named ```index``` at the target directory, if the ```index``` file does not exist, ```404 NOT FOUND``` is returned.

## Request Mapping

Request mapping allow the override of the default request path to file path mapping behavior. If the requested path exists in request map, the content of the mapped file is used as the response instead.

To map a request path to a file, create a ```map.txt``` file at the ```PWD```. The format is ```${req_path}=${file_path}```. 

For example, ```/req=res.txt``` maps the request path ```/req``` to the file ```./res/res.txt```

Sample File:

```
/req1=res1.txt
/res2=res2/res.txt
```