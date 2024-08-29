# HTTP SERVER

A http file server. The program reads the file content based on the path given in a HTTP GET request and returns its content. Useful for http service / API mocking.

## Compilation

To build the executable. Run the following command

```rust
cargo build --release
```

## Running the program

The program can be run using the below command. 

- ```port``` is the port number to listen for (Default to 3006)
- ```root_folder``` is the root folder of the prgoram (Default to the executable file's parent)

### Linux / Mac

```bash
./http-server [-p <port>] [-f <root_folder>]
```

### Windows

```powershell
.\http-server.exe [-p <port>] [-f <root_folder>]
```

## HTTP Response

Responses are created by reading files within the ```res``` folder using relative path. There should be a folder named ```res``` in the ```root_folder```.

For example, a request for path ```/inner/res``` will be mapped to the file ```<root_folder>/res/inner/res``` (no file extension). If the file does not exists, ```404 NOT FOUND``` is returned.

If the mapped file_path is a directory, a read attempt is made to the file named ```index``` at the target directory, if the ```index``` file does not exist, ```404 NOT FOUND``` is returned.

## Request Mapping

Request mapping allow the override of the default request path to file path mapping behavior. If the requested path exists in request map, the content of the mapped file is used as the response instead.

To use request mapping, create a ```map.txt``` file at the ```root_folder```.

There are two types of mapping. One to one request map and one to many request map

### Single Request Map (One to One)

To map a request path to a single file, make an entry to the map file with the format of ```${req_path} = ${file_path}```. 

For example, ```/req = res.txt``` maps the request path ```/req``` to the file ```<root_folder>/res/res.txt```

### Multi Request Map (One to Many)

One request path could be mapped to multiple file paths. Each file path has its corresponding relative weight.

To map a request path to multiple files, make an entry to the map file with the format of ```${req_path} = ${file_path}'${weight}[, ]```

For example, ```/req = res1.txt'30, res2.txt'70``` will map the request path ```<root_folder>/req``` to ```res1.txt``` with a ```weight``` of 30 and ```res2.txt``` with a ```weight``` of 70.

When the path is requested, the file path is chosen randomly based on the weight of each provided path. Each ```Weight``` has to be a ***non-zero positive integer***.

### Sample File:

```
/req1 = res1.txt
/res2 = res2.txt'50, res3.txt'50
```