# needed for sqlx
cargo update;
# export DATABASE_URL=postgres://postgres:eJk16bVgFNkJI74s3uY248vwCX7rEkUbGXrZtS8V4PDn8e2HcC@localhost:54320/alpaca
make sqlx;
git add .;
git commit -am "sqlx";
source ~/.bashrc;
git push origin dev;
ssh swimr205 "cd dev/rust/_trade/trade; export GIT_SSH_COMMAND='ssh -i /home/glp/key/ssh_github_20220411/ssh_github_20220411'; git pull origin dev; make -B frontend; docker logs --follow --tail 100 frontend"
# make sqlx; git add .;git commit -am "sqlx"; source ~/.bashrc; git push origin dev; ssh swimr205remote "cd dev/rust/_trade/trade; export GIT_SSH_COMMAND='ssh -i /home/glp/key/ssh_github_20220411/ssh_github_20220411'; git pull origin dev; make -B frontend; docker logs --follow --tail 100 frontend"

