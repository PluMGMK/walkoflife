# walkoflife
A little Rust program for Linux to debug the dodgy timing in the Ly races in Rayman 2, as discussed [here](https://raymanpc.com/forum/viewtopic.php?p=1431044#p1431044).

You can run it while Rayman 2 is running, and in the Walk of Life level. It'll keep running until you quit the level, query the game every second and print out a line of the form:
```
<COUNTDOWN> -> <TIMER>
```
Where `<COUNTDOWN>` is the number of seconds before the level times out (as currently displayed on the screen), and `<TIMER>` is the game's internal tracker of how long you've been racing (in milliseconds).

Much of the program logic comes from [Robin's Rayman 2 fun box](https://github.com/rtsonneveld/Rayman2FunBox) - without him, this wouldn't have been possible.
