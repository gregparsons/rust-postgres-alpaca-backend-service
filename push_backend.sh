make sqlx; 
git add .;
git commit -am "sqlx"; 
source ~/.bashrc; git push origin dev; ssh swimr205 "cd dev/rust/_trade/trade;
export GIT_SSH_COMMAND='ssh -i /home/glp/key/ssh_github_20220411/ssh_github_20220411'; 
git pull origin dev;
make -B backend"

# swimr205remote
#make sqlx; git add .;git commit -am "sqlx"; source ~/.bashrc; git push origin dev; ssh swimr205remote "cd dev/rust/_trade/trade; export GIT_SSH_COMMAND='ssh -i /home/glp/key/ssh_github_20220411/ssh_github_20220411'; git pull origin dev;make -B backend"

