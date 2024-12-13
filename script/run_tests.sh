vagrant up
vagrant ssh -c "cd /vagrant; script/setup.sh; cargo test"
vagrant halt
