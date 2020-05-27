Upload a file
```bash
curl -i -X POST -H "Content-Type: multipart/form-data" -F "data=@myfile.bin" http://localhost:8080/upload
```