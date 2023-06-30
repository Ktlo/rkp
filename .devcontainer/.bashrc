#!/usr/bin/env bash

PS1='${debian_chroot:+($debian_chroot)}\[\033[01;91m\]\u@rkp:\[\033[01;34m\]\w\[\033[00m\]\$ '
umask 022
export LS_OPTIONS='--color=auto'
eval "`dircolors`"
alias ls='ls $LS_OPTIONS'
alias editor=code
export EDITOR=code
