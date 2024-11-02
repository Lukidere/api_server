#!/bin/bash
cd /opt/lampp || exit

sudo ./xampp startmysql && sudo ./xampp startapache
