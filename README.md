# Kwark

An editor `framework` that allows you to configure your editor as if it were just a library

## Getting Started
Kwark has a full `book` that shows you how to use it, and explains the concepts that kwark follows.

You can find it at [theemeraldbee.github.io/kwark/book](https://theemeraldbee.github.io/kwark/book/).
Enjoy the read! And hopefully you can find your new editor.

## Is Kwark an Editor
Yes and No, kwark is an editor in the sense that, with very *minimal* "configuration", you
can have a fully functional editor that would easily fit as your daily driver. However,
kwark on its own is not an editor that you can just install. You **must** build a rust project
(or find someone else's configuration online), and then run that project in order to have an editor.

## Why do it this way?
I've always hated trying to work with Scripting languages for an editor whose full implementation
could be laid out before me, it's open source! Why not just edit the source? So I had the goal to fix this.
Kwark came about from many, many attempts at trying to make my own editor, and finally I settled on this format.

## Why should I use it?
Kwark is the **most** customizable editor you'll ever see, because it's a framework. You can literally do
pretty much whatever you want with the editor, from setting up your own keybinds from **no** base, to
even changing the way every buffer renders, what Gutters are drawn.

However, while Kwark is incredibly customizable, Kwark will always have the goal to have an "out of the box"
feeling, by following just a little bit of the book, or by copying one of the example configurations, you
can quickly build out a good feeling editor, that, when you feel like adjusting it, you can do so very easily

## Why shouldn't I use it?
Kwark has a goal of being a "fits-all" solution to text editing, however,
you have to have a grasp of rust in order to use the configuration effectively.
If this is a turnoff, I understand, and I'd highly recommend looking at `helix`, as it comes out of
the box fitting many users.
