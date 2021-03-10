import numpy as np
import matplotlib.pyplot as plt

def plot_advect1d():
    x, p = np.loadtxt('solution.dat').T
    fig = plt.figure(figsize=[10, 10])
    ax1 = fig.add_subplot(1, 1, 1)
    ax1.plot(x, p, '-o', mfc='none')
    plt.show()

def plot_advect1d_blocks():
    import glob
    fig = plt.figure(figsize=[10, 10])
    ax1 = fig.add_subplot(1, 1, 1)
    for filename in glob.glob("solution-*.dat"):
        x, p = np.loadtxt(filename).T
        ax1.plot(x, p, '-o', mfc='none')
    plt.show()

plot_advect1d_blocks()
