export DISPLAY=:99.0
Xvfb :99.0 &
sleep 3
i3 -c /dev/null &
sleep 3
gpick &
sleep 3
xterm &
sleep 3
DISPLAY=:99.0 i3-msg [class="XTerm"] floating enable
