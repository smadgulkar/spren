@echo off 
echo CPU Information: 
wmic cpu get name, maxclockspeed, numberofcores, status 
echo. 
echo RAM Information: 
wmic memorychip get capacity, devicelocator, manufacturer 
echo. 
echo Storage Information: 
wmic diskdrive get caption, size, status 
