#!/bin/sh
mkdir -p /home/foxfire/.config/REAPER
cat > /home/foxfire/.config/REAPER/reaper.ini <<'INI'
[REAPER]
vstpath=/home/foxfire/.paseo/worktrees/13j33zef/parched-goose/target/bundled
vstpath64=/home/foxfire/.paseo/worktrees/13j33zef/parched-goose/target/bundled
wnd_w=1200
wnd_h=800
INI
cp /home/foxfire/.paseo/worktrees/13j33zef/parched-goose/evidence/editor-lifecycle/reaper/artifacts/editor-regression.rpp /home/foxfire/.artifacts/editor-regression.rpp
exec /usr/bin/reaper -newinst -nosplash /home/foxfire/.artifacts/editor-regression.rpp
