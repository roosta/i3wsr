vagrant up
vagrant ssh -c "cd /vagrant; script/setup.sh; cargo clean; cargo test"
vagrant halt
