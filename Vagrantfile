VAGRANTFILE_API_VERSION = '2'

Vagrant.configure(VAGRANTFILE_API_VERSION) do |config|
  config.vm.define :ubuntu do |ubuntu|
    ubuntu.vm.box = 'ubuntu/lunar64'
    ubuntu.vm.provision 'shell', path: 'script/vagrant_root.sh'
    ubuntu.vm.provision 'shell', privileged: false, path: 'script/vagrant_user.sh'
  end
end
